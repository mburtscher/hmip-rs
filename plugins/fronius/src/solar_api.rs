use std::time::Duration;

use hmip_rs::domain::Device;
use hmip_rs::domain::features::{BatteryStateFeature, CurrentPowerFeature};
use hmip_rs::domain::types::DeviceType;
use serde::Deserialize;

#[cfg(debug_assertions)]
mod ids {
    pub const INVERTER: &str = "1a194539-9be7-44c4-baec-a5585cf031c5";
    pub const BATTERY: &str = "898a1a50-e9c8-4c1e-bbb5-0ffb3b57cfa6";
    pub const GRID_CONNECTION: &str = "53be7bff-1f3e-4b0e-9b9e-5538f0110507";
    pub const METER: &str = "a56d607b-9a94-4975-bb5a-9f92a73e404f";
}
#[cfg(not(debug_assertions))]
mod ids {
    pub const INVERTER: &str = "a9d52709-ae08-46e2-83e3-5266f9e59d82";
    pub const BATTERY: &str = "dd70665e-7e7f-412e-9e1f-cc58ba8f028c";
    pub const GRID_CONNECTION: &str = "497a45f5-439e-4a48-8cd9-944910129ff0";
    pub const METER: &str = "bf207122-c538-4b14-a189-65bbf3ebbe17";
}

/// Realtime power flow as reported by `GetPowerFlowRealtimeData.fcgi`.
///
/// Values the inverter does not provide (e.g. no battery installed) are `null` in the response
/// and therefore `None` here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PowerFlow {
    /// Photovoltaic production in W.
    pub pv: Option<f64>,
    /// Battery power in W; positive while discharging.
    pub battery: Option<f64>,
    /// Grid power in W; positive while importing.
    pub grid: Option<f64>,
    /// House consumption in W; negative by Fronius convention.
    pub load: Option<f64>,
    /// Battery state of charge in percent.
    pub state_of_charge: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Response {
    body: ResponseBody,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ResponseBody {
    data: Data,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Data {
    site: Site,
    #[serde(default)]
    inverters: std::collections::HashMap<String, InverterData>,
}

#[derive(Deserialize)]
struct Site {
    #[serde(rename = "P_PV")]
    pv: Option<f64>,
    #[serde(rename = "P_Akku")]
    battery: Option<f64>,
    #[serde(rename = "P_Grid")]
    grid: Option<f64>,
    #[serde(rename = "P_Load")]
    load: Option<f64>,
}

#[derive(Deserialize)]
struct InverterData {
    #[serde(rename = "SOC")]
    soc: Option<f64>,
}

impl PowerFlow {
    /// Parses a Solar API response body.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let Response { body } = serde_json::from_str(text)?;
        let Data { site, inverters } = body.data;
        Ok(Self {
            pv: site.pv,
            battery: site.battery,
            grid: site.grid,
            load: site.load,
            state_of_charge: inverters.get("1").and_then(|i| i.soc),
        })
    }

    /// Maps the power flow onto the plugin's four devices.
    ///
    /// Power is reported from the home's point of view: production, battery discharge and house
    /// consumption are negative where the Fronius API reports them positive (and vice versa).
    pub fn devices(&self) -> Vec<Device> {
        let power = |value: Option<f64>| CurrentPowerFeature {
            current_power: value,
        };
        vec![
            Device::with_id(ids::INVERTER, DeviceType::Inverter)
                .friendly_name("Wechselrichter")
                .model_type("Fronius SymoGEN24")
                .firmware_version("?.?.?")
                .feature(power(self.pv.map(|v| -v))),
            Device::with_id(ids::BATTERY, DeviceType::Battery)
                .friendly_name("Batterie")
                .model_type("BYD")
                .firmware_version("?.?.?")
                .feature(BatteryStateFeature {
                    battery_level: self.state_of_charge.map(|soc| soc / 100.0),
                    battery_capacity: None,
                })
                .feature(power(self.battery.map(|v| -v))),
            Device::with_id(ids::GRID_CONNECTION, DeviceType::GridConnectionPoint)
                .friendly_name("Netzzugang")
                .model_type("Fronius Power Meter")
                .firmware_version("?.?.?")
                .feature(power(self.grid)),
            Device::with_id(ids::METER, DeviceType::EnergyMeter)
                .friendly_name("Hausverbrauch")
                .model_type("Fronius Power Meter")
                .firmware_version("?.?.?")
                .feature(power(self.load.map(|v| -v))),
        ]
    }
}

/// Client for the Fronius Solar API.
#[derive(Debug, Clone)]
pub struct SolarApiClient {
    http: reqwest::Client,
}

impl Default for SolarApiClient {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("static client configuration is valid"),
        }
    }
}

impl SolarApiClient {
    fn url(host: &str) -> String {
        format!("http://{host}/solar_api/v1/GetPowerFlowRealtimeData.fcgi")
    }

    /// Fetches the current power flow from the inverter at `host`.
    pub async fn power_flow(
        &self,
        host: &str,
    ) -> Result<PowerFlow, Box<dyn std::error::Error + Send + Sync>> {
        let text = self
            .http
            .get(Self::url(host))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(PowerFlow::from_json(&text)?)
    }

    /// Whether the Solar API endpoint answers successfully under `host`.
    pub async fn is_available(&self, host: &str) -> bool {
        match self.http.get(Self::url(host)).send().await {
            Ok(response) => response.status().is_success(),
            Err(err) => {
                tracing::warn!("API endpoint not available under {host}: {err}");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "Body": {"Data": {
            "Inverters": {"1": {"DT": 1, "P": 2500, "SOC": 55.5}},
            "Site": {"P_Akku": 300.4, "P_Grid": -1200.0, "P_Load": -1600, "P_PV": 2500.2, "Mode": "bidirectional"}
        }},
        "Head": {"Status": {"Code": 0}}
    }"#;

    #[test]
    fn parses_the_power_flow() {
        let flow = PowerFlow::from_json(SAMPLE).unwrap();
        assert_eq!(flow.pv, Some(2500.2));
        assert_eq!(flow.battery, Some(300.4));
        assert_eq!(flow.grid, Some(-1200.0));
        assert_eq!(flow.load, Some(-1600.0));
        assert_eq!(flow.state_of_charge, Some(55.5));
    }

    #[test]
    fn null_values_stay_unset() {
        let flow = PowerFlow::from_json(
            r#"{"Body":{"Data":{"Inverters":{},"Site":{"P_PV":10,"P_Akku":null,"P_Grid":null,"P_Load":-5}}}}"#,
        )
        .unwrap();
        assert_eq!(flow.battery, None);
        assert_eq!(flow.state_of_charge, None);
        assert!(flow.devices().iter().all(|d| d.validate().is_ok()));
    }

    #[test]
    fn devices_use_the_home_point_of_view_and_are_valid() {
        let devices = PowerFlow::from_json(SAMPLE).unwrap().devices();
        assert_eq!(devices.len(), 4);
        let power = |i: usize| match devices[i].features.last().unwrap() {
            hmip_rs::domain::Feature::CurrentPower(f) => f.current_power.unwrap(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(power(0), -2500.2); // inverter produces
        assert_eq!(power(1), -300.4); // battery discharges
        assert_eq!(power(2), -1200.0); // grid
        assert_eq!(power(3), 1600.0); // load
        assert!(devices.iter().all(|d| d.validate().is_ok()));
    }

    #[tokio::test]
    async fn queries_a_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(
                    String::from_utf8_lossy(&buf[..n])
                        .starts_with("GET /solar_api/v1/GetPowerFlowRealtimeData.fcgi")
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{SAMPLE}",
                    SAMPLE.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let client = SolarApiClient::default();
        assert!(client.is_available(&host).await);
        assert_eq!(client.power_flow(&host).await.unwrap().pv, Some(2500.2));
    }
}
