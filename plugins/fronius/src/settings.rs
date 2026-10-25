use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Persistent plugin configuration. Stored as `/data/config.json`, which the HCU keeps across
/// updates and restarts. The key spelling matches the .NET implementation's file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(rename = "InverterIpAddress")]
    pub inverter_ip_address: Option<String>,
}

impl Settings {
    /// `/data/config.json`, overridable with `FRONIUS_CONFIG` for local development.
    pub fn default_path() -> PathBuf {
        std::env::var_os("FRONIUS_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/data/config.json"))
    }

    /// Loads the settings; a missing or unreadable file yields the defaults.
    pub async fn load(path: &Path) -> Self {
        match tokio::fs::read_to_string(path).await {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
                tracing::warn!("ignoring invalid {}: {err}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Writes the settings, creating the parent directory if needed.
    pub async fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        tokio::fs::write(path, serde_json::to_vec(self)?).await
    }

    /// Whether an inverter address has been configured.
    pub fn is_configured(&self) -> bool {
        self.inverter_ip_address
            .as_deref()
            .is_some_and(|ip| !ip.is_empty())
    }
}

/// The configuration form only accepts dotted IPv4 addresses (see the template's pattern).
pub fn validate_ip_address(value: Option<&str>) -> Result<&str, String> {
    let value = value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or("Inverter IP address is required.")?;
    value
        .parse::<Ipv4Addr>()
        .map(|_| value)
        .map_err(|_| format!("'{value}' is not a valid IPv4 address."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ipv4_addresses() {
        assert_eq!(
            validate_ip_address(Some(" 192.168.1.20 ")),
            Ok("192.168.1.20")
        );
        assert!(validate_ip_address(Some("300.1.1.1")).is_err());
        assert!(validate_ip_address(Some("inverter.local")).is_err());
        assert!(validate_ip_address(Some("")).is_err());
        assert!(validate_ip_address(None).is_err());
    }

    #[tokio::test]
    async fn roundtrips_through_the_dotnet_compatible_file() {
        let dir = std::env::temp_dir().join(format!("fronius-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let settings = Settings {
            inverter_ip_address: Some("10.0.0.5".into()),
        };
        settings.save(&path).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"InverterIpAddress":"10.0.0.5"}"#
        );
        assert_eq!(Settings::load(&path).await, settings);
        assert_eq!(
            Settings::load(&dir.join("missing.json")).await,
            Settings::default()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
