//! HomematicIP plugin for Fronius SYMO Gen24 inverters.

mod plugin;
mod settings;
mod solar_api;

use hmip_rs::Plugin;
use hmip_rs::domain::types::PluginReadinessStatus;

use crate::plugin::FroniusPlugin;
use crate::settings::Settings;

/// Must match `package.metadata.hcu.plugin-id` in Cargo.toml (checked by a test).
#[cfg_attr(debug_assertions, allow(dead_code))]
const RELEASE_PLUGIN_ID: &str = "xyz.burtscher.homematic.plugin.fronius";
/// Debug builds register as a separate plugin so they never collide with an installed one.
#[cfg(debug_assertions)]
const PLUGIN_ID: &str = "xyz.burtscher.homematic.plugin.fronius.debug";
#[cfg(not(debug_assertions))]
const PLUGIN_ID: &str = RELEASE_PLUGIN_ID;

#[tokio::main]
async fn main() -> hmip_rs::Result<()> {
    // `RUST_LOG` takes a plain level (error, warn, info, debug, trace); default is info.
    tracing_subscriber::fmt()
        .with_max_level(
            std::env::var("RUST_LOG")
                .ok()
                .and_then(|level| level.parse::<tracing::Level>().ok())
                .unwrap_or(tracing::Level::INFO),
        )
        .init();

    let config_path = Settings::default_path();
    let settings = Settings::load(&config_path).await;

    // Installed plugins use the defaults; set HCU_HOST and HCU_TOKEN to run remotely during
    // development (see the Connect API documentation, "Getting Started").
    let mut builder = Plugin::builder(PLUGIN_ID)
        .friendly_name("de", "Fronius Plugin")
        .friendly_name("en", "Fronius Plugin")
        .initial_readiness(if settings.is_configured() {
            PluginReadinessStatus::Ready
        } else {
            PluginReadinessStatus::ConfigRequired
        });
    if let Ok(host) = std::env::var("HCU_HOST") {
        builder = builder.host(host);
    }
    if let Ok(token) = std::env::var("HCU_TOKEN") {
        builder = builder.token(token);
    }

    builder
        .build()
        .run(FroniusPlugin::new(config_path, settings))
        .await
}

#[cfg(test)]
mod tests {
    #[test]
    fn plugin_id_matches_the_image_metadata() {
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains(&format!("plugin-id = \"{}\"", super::RELEASE_PLUGIN_ID)));
    }
}
