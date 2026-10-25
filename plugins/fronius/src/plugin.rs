use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use hmip_rs::api::*;
use hmip_rs::domain::types::{ConfigUpdateResponseStatus, PluginReadinessStatus, PropertyType};
use hmip_rs::domain::{Device, ErrorInfo, GroupTemplate, PropertyTemplate};
use hmip_rs::{Context, Handler, Result};
use tokio::sync::RwLock;

use crate::settings::{Settings, validate_ip_address};
use crate::solar_api::SolarApiClient;

/// How often the current state is pushed to the HCU.
const PUSH_INTERVAL: Duration = Duration::from_secs(10);

const IP_PATTERN: &str =
    r"^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$";

pub struct FroniusPlugin {
    config_path: PathBuf,
    settings: RwLock<Settings>,
    client: SolarApiClient,
}

impl FroniusPlugin {
    pub fn new(config_path: PathBuf, settings: Settings) -> Self {
        Self {
            config_path,
            settings: RwLock::new(settings),
            client: SolarApiClient::default(),
        }
    }

    /// Current devices, or `None` if no inverter is configured. The error is a ready-to-send
    /// description of why the Solar API could not be queried.
    async fn devices(&self) -> std::result::Result<Vec<Device>, ErrorInfo> {
        let host = self
            .settings
            .read()
            .await
            .inverter_ip_address
            .clone()
            .filter(|h| !h.is_empty());
        let Some(host) = host else {
            return Err(
                ErrorInfo::new("CONFIG_REQUIRED").message("No inverter IP address configured.")
            );
        };
        match self.client.power_flow(&host).await {
            Ok(flow) => Ok(flow.devices()),
            Err(err) => Err(ErrorInfo::new("SOLAR_API_UNAVAILABLE")
                .message(format!("Could not read the Solar API under {host}: {err}"))),
        }
    }

    async fn status_response(&self, only: Option<&[String]>) -> StatusResponseBody {
        match self.devices().await {
            Ok(mut devices) => {
                if let Some(ids) = only {
                    devices.retain(|d| ids.contains(&d.device_id));
                }
                StatusResponseBody {
                    success: true,
                    devices,
                    error: None,
                }
            }
            Err(error) => StatusResponseBody {
                success: false,
                devices: Vec::new(),
                error: Some(error),
            },
        }
    }
}

impl Handler for FroniusPlugin {
    /// Pushes the state of all devices periodically while connected.
    async fn on_connected(&self, ctx: &Context) -> Result<()> {
        let mut interval = tokio::time::interval(PUSH_INTERVAL);
        loop {
            interval.tick().await;
            if !self.settings.read().await.is_configured() {
                continue;
            }
            let status = self.status_response(None).await;
            if status.success {
                ctx.send(status)?;
            } else {
                tracing::warn!("skipping periodic update: {:?}", status.error);
            }
        }
    }

    async fn on_config_template_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        _: ConfigTemplateRequestBody,
    ) -> Result<()> {
        let current = self.settings.read().await.inverter_ip_address.clone();
        let mut ip = PropertyTemplate::new(PropertyType::String, "IP-Adresse des Wechselrichters");
        ip.description = Some(
            "Unter dieser IP-Adresse muss der Endpunkt http://IP_ADRESSE/solar_api/v1/GetPowerFlowRealtimeData.fcgi \
             verfügbar sein. Anleitung zur Aktivierung der Solar API: https://www.youtube.com/watch?v=WHu6e-6cEUU"
                .into(),
        );
        ip.group_id = Some("api".into());
        ip.current_value = current;
        ip.pattern = Some(IP_PATTERN.into());
        ip.required = Some(true);

        ctx.reply(
            id,
            ConfigTemplateResponseBody {
                groups: Some(HashMap::from([(
                    "api".to_owned(),
                    GroupTemplate {
                        friendly_name: "Solar API".into(),
                        ..Default::default()
                    },
                )])),
                properties: HashMap::from([("inverterIpAddress".to_owned(), ip)]),
            },
        )
    }

    async fn on_config_update_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        body: ConfigUpdateRequestBody,
    ) -> Result<()> {
        let requested = match body
            .properties
            .as_ref()
            .and_then(|p| p.get("inverterIpAddress"))
        {
            Some(value) => value.as_str().map(str::to_owned),
            // Unchanged: keep the current value.
            None => self.settings.read().await.inverter_ip_address.clone(),
        };

        let outcome = match validate_ip_address(requested.as_deref()) {
            Err(message) => Err(message),
            Ok(ip) if !self.client.is_available(ip).await => {
                Err(format!("API endpoint not available under {ip}."))
            }
            Ok(ip) => Ok(ip.to_owned()),
        };

        let response = match outcome {
            Ok(ip) => {
                let mut settings = self.settings.write().await;
                settings.inverter_ip_address = Some(ip);
                match settings.save(&self.config_path).await {
                    Ok(()) => {
                        drop(settings);
                        ctx.set_readiness(PluginReadinessStatus::Ready)?;
                        ConfigUpdateResponseBody {
                            status: ConfigUpdateResponseStatus::Applied,
                            message: None,
                        }
                    }
                    Err(err) => ConfigUpdateResponseBody {
                        status: ConfigUpdateResponseStatus::Failed,
                        message: Some(format!("Could not save the configuration: {err}")),
                    },
                }
            }
            Err(message) => ConfigUpdateResponseBody {
                status: ConfigUpdateResponseStatus::Failed,
                message: Some(message),
            },
        };
        ctx.reply(id, response)
    }

    async fn on_discover_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        _: DiscoverRequestBody,
    ) -> Result<()> {
        let body = match self.devices().await {
            Ok(devices) => DiscoverResponseBody {
                success: true,
                devices,
                error: None,
            },
            Err(error) => DiscoverResponseBody {
                success: false,
                devices: Vec::new(),
                error: Some(error),
            },
        };
        ctx.reply(id, body)
    }

    async fn on_inclusion_event(
        &self,
        ctx: &Context,
        id: Option<String>,
        body: InclusionEventBody,
    ) -> Result<()> {
        ctx.reply(id, self.status_response(Some(&body.device_ids)).await)
    }

    async fn on_status_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        body: StatusRequestBody,
    ) -> Result<()> {
        ctx.reply(id, self.status_response(body.device_ids.as_deref()).await)
    }
}
