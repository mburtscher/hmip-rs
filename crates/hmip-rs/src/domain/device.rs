use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::features::Feature;
use super::types::DeviceType;
use crate::error::Result;

/// A device exposed by a plugin to the Home Control Unit.
///
/// Create devices with [`Device::new`] (random id) or [`Device::with_id`] (stable id – use this for
/// real devices, the id must survive plugin restarts) and add [features](super::features) with
/// [`Device::feature`]:
///
/// ```
/// use hmip_rs::domain::{Device, features::*, types::DeviceType};
///
/// let light = Device::with_id("kitchen-light", DeviceType::Light)
///     .friendly_name("Kitchen")
///     .model_type("LTA004")
///     .firmware_version("1.2.3")
///     .feature(SwitchStateFeature { on: Some(true) })
///     .feature(DimmingFeature { dim_level: Some(0.5) });
///
/// // Checks value ranges, required features and features allowed for the archetype.
/// light.validate().unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// Unique identifier of the device.
    pub device_id: String,
    /// The device archetype; determines the required and optional features.
    pub device_type: DeviceType,
    /// A vendor specific model type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_type: Option<String>,
    /// A human readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<String>,
    /// The firmware version running on the device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
    /// The features and their values.
    #[serde(default)]
    pub features: Vec<Feature>,
}

impl Device {
    /// Creates a device with a fresh random id and no features.
    pub fn new(device_type: DeviceType) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), device_type)
    }

    /// Creates a device with the given id and no features.
    pub fn with_id(device_id: impl Into<String>, device_type: DeviceType) -> Self {
        Self {
            device_id: device_id.into(),
            device_type,
            model_type: None,
            friendly_name: None,
            firmware_version: None,
            features: Vec::new(),
        }
    }

    /// Sets the model type.
    pub fn model_type(mut self, value: impl Into<String>) -> Self {
        self.model_type = Some(value.into());
        self
    }

    /// Sets the friendly name.
    pub fn friendly_name(mut self, value: impl Into<String>) -> Self {
        self.friendly_name = Some(value.into());
        self
    }

    /// Sets the firmware version.
    pub fn firmware_version(mut self, value: impl Into<String>) -> Self {
        self.firmware_version = Some(value.into());
        self
    }

    /// Adds a feature.
    pub fn feature(mut self, feature: impl Into<Feature>) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Checks the value ranges of all features and that the features fit the device type: every
    /// required feature of the archetype must be present and nothing outside the required and
    /// optional features may be used.
    pub fn validate(&self) -> Result<()> {
        self.features.iter().try_for_each(Feature::validate)?;

        let required = self.device_type.required_features();
        let optional = self.device_type.optional_features();
        for feature in &self.features {
            let name = feature.type_name();
            if !required.contains(&name) && !optional.contains(&name) {
                return Err(crate::Error::FeatureNotSupported {
                    device_type: self.device_type,
                    feature: name.to_owned(),
                });
            }
        }
        for name in required {
            if !self.features.iter().any(|f| f.type_name() == *name) {
                return Err(crate::Error::MissingFeature {
                    device_type: self.device_type,
                    feature: (*name).to_owned(),
                });
            }
        }
        Ok(())
    }
}

/// Error details included in responses, e.g. in
/// [`StatusResponseBody`](crate::api::StatusResponseBody) or
/// [`ControlResponseBody`](crate::api::ControlResponseBody).
///
/// ```
/// use hmip_rs::domain::ErrorInfo;
///
/// let error = ErrorInfo::new("BRIDGE_OFFLINE").message("The bridge does not respond.");
/// assert_eq!(error.code, "BRIDGE_OFFLINE");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInfo {
    /// A code categorizing the error, typically a number or an enumerated type.
    pub code: String,
    /// An optional message with additional details or context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ErrorInfo {
    /// Creates an error with a code and no message.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: None,
        }
    }

    /// Adds a human readable message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}
