//! Domain model: devices, their features and configuration templates.
//!
//! * [`Device`] – a plugin device: id, [`DeviceType`](types::DeviceType) archetype, names and a
//!   list of [features].
//! * [`features`] – the 40 feature structs and the [`Feature`] union that carries them.
//! * [`types`] – enumerations such as `DeviceType`, `PluginReadinessStatus` or `PropertyType`.
//! * [`GroupTemplate`] / [`PropertyTemplate`] – describe the configuration form HCUweb shows.
//! * [`ErrorInfo`] – error details reported in responses.
//!
//! ```
//! use hmip_rs::domain::{Device, features::SwitchStateFeature, types::DeviceType};
//!
//! let plug = Device::new(DeviceType::Switch)
//!     .friendly_name("Coffee machine")
//!     .feature(SwitchStateFeature { on: Some(true) });
//! plug.validate().unwrap(); // a SWITCH requires a switchState feature
//! ```

mod device;
pub mod features;
mod template;
pub mod types;

pub use device::{Device, ErrorInfo};
pub use features::Feature;
pub use template::{GroupTemplate, PropertyTemplate};
