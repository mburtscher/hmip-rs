//! Enumerated types shared by the Connect API messages.
//!
//! All enums serialize to the `SCREAMING_SNAKE_CASE` strings of the API (`DeviceType::EvCharger` is
//! `"EV_CHARGER"`) and are `#[non_exhaustive]`, so new values in future API versions are not a
//! breaking change.

use serde::{Deserialize, Serialize};

/// Acknowledgement type of a user message created by a plugin to be shown in the Homematic IP smartphone app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum AckType {
    /// User acknowledged the message with NO. Applicable for user messages with BehaviorType 'ACKNOWLEDGEABLE_BY_YES_NO'.
    No,
    /// User acknowledged the message with OK. Applicable for user messages with BehaviorType 'ACKNOWLEDGEABLE_BY_OK'.
    Ok,
    /// User acknowledged the message with YES. Applicable for user messages with BehaviorType 'ACKNOWLEDGEABLE_BY_YES_NO'.
    Yes,
}

/// Behavior type of a user message created by a plugin to be shown in the Homematic IP smartphone app. Used by app to enable the user to dismiss or acknowledge the message, or to prevent the user from dismissing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum BehaviorType {
    /// Users may acknowledge and thereby dismiss the user message in the Homematic IP smartphone app. The plugin will receive an acknowledgement event with AckType OK when the message is acknowledged.
    AcknowledgeableByOk,
    /// Users may acknowledge the user message by selecting YES or NO in the Homematic IP smartphone app. The plugin will receive an acknowledgement event with AckType YES or NO when the message is acknowledged. The plugin may then dismiss the user message by sending a delete user message request.
    AcknowledgeableByYesNo,
    /// Users may dismiss the user message in the Homematic IP smartphone app. The plugin will not receive an acknowledgement event when the message is dismissed.
    Dismissible,
    /// Users may not dismiss the user message in the Homematic IP smartphone app.
    NotDismissible,
}

/// Enumerated type of climate operation modes. Used by plugin devices supporting the ClimateOperationMode feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ClimateOperationType {
    /// The device decides between heating and cooling.
    Auto,
    /// The device cools.
    Cooling,
    /// The device heats.
    Heating,
}

/// Describes the result or intermediate status of a plugin configuration update that was triggered by a HCUweb user by updating the plugin configuration and saving the changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ConfigUpdateResponseStatus {
    /// The plugin configuration update was applied successfully.
    Applied,
    /// The plugin configuration update could not be applied.
    Failed,
    /// The plugin configuration update is still pending, e.g. because applying it takes time or requires user interaction.
    Pending,
}

/// Enumerated type of the plugin device archetypes. Each type supports a set of required and/or optional features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum DeviceType {
    /// Type for batteries.
    Battery,
    /// Type for sensors measuring or detecting parameters of the environment, like the current temperature or whether it rains or not.
    ClimateSensor,
    /// Type for contact sensors detecting, for example, if a window is opened or closed.
    ContactSensor,
    /// Type for devices monitoring the energy input and output of other devices.
    EnergyMeter,
    /// Type for electric vehicle chargers.
    EvCharger,
    /// Type for grid connection points, i.e. the device that connects your home to the electrical grid.
    GridConnectionPoint,
    /// Type for heat pumps.
    HeatPump,
    /// Type for air conditioning devices.
    Hvac,
    /// Type for inverters like solar power inverters.
    Inverter,
    /// Type for lighting devices like light bulbs or light strips.
    Light,
    /// Type for devices like motion sensors that can detect presence in a room or area.
    OccupancySensor,
    /// Type for sensors that measure particulate matter in the environment.
    ParticulateMatterSensor,
    /// Type for smoke detecting devices.
    SmokeAlarm,
    /// Type for device that can be switched on or off.
    Switch,
    /// Type for devices that can be triggered to send out single switch signals to the system, like buttons.
    SwitchInput,
    /// Type for thermostats, i.e. devices that can heat or cool to a setpoint temperature.
    Thermostat,
    /// Type for electric vehicles.
    Vehicle,
    /// Type for water sensors.
    WaterSensor,
    /// Type for window covering devices like electric window shutters.
    WindowCovering,
}

/// Category of a user message created by a plugin to be shown in the Homematic IP smartphone app. Used by the app to show the message with applicable color and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MessageCategory {
    /// An error; shown in the error color.
    Error,
    /// Informational.
    Info,
    /// A warning.
    Warn,
}

/// Enumerated type of the plugin state. Included in a PluginStateResponse sent by the plugin to the Home Control Unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum PluginReadinessStatus {
    /// The plugin has to be configured to work as intended. This status should be used to communicate to users that they have to configure the plugin via HCUweb.
    ConfigRequired,
    /// The plugin could not be started, or any other unexpected error occurred. This status should be used to communicate to users that the plugin is not functioning as expected.
    Error,
    /// The plugin is up and running. This status should be used to communicate to users that the plugin is ready for usage, typically right after startup or after successfully applying a configuration update.
    Ready,
}

/// Enumerated type of presence modes. Used by plugin devices supporting the PresenceMode feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum PresenceType {
    /// Nobody is at home.
    Away,
    /// The default mode.
    Default,
    /// Somebody is at home.
    Home,
    /// The normal mode.
    Normal,
    /// Away for an extended period.
    Vacation,
}

/// Enumerated data type of a plugin configuration property defined in a property template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum PropertyType {
    /// Type for boolean properties.
    Boolean,
    /// Type for property with a predefined set of values. In the property template, use the field 'values' to provide the set of possible values.
    Enum,
    /// Type for integer properties. In the property template, use the fields 'minimum' and 'maximum' to define constraints.
    Integer,
    /// Type for floating point number properties. In the property template, use the fields 'minimum' and 'maximum' to define constraints.
    Number,
    /// Type for password properties. Will be displayed obscured.
    Password,
    /// Type for QR code properties. In the property template, use the field 'currentValue' for the text used to generate the QR code.
    Qrcode,
    /// Type for string properties that cannot be modified by user. In the property template, use the field 'currentValue' for the read-only text.
    Readonly,
    /// Type for string properties. In the property template, use the fields 'minimumLength', 'maximumLength' and 'pattern' to define constraints.
    String,
    /// Type for string properties with autocompletion. In the property template, use the field 'values' for a list of possible values the user may select while typing.
    Typeahead,
    /// Type for for weblink properties. In the property template, use the field 'currentValue' for the link and the field 'defaultValue' for additional info text.
    Weblink,
}

/// Enumerated type of shading directions. Used by plugin devices supporting the ShutterDirection feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ShadingDirection {
    /// The shutter moved down (darker).
    Darker,
    /// The shutter moved up (lighter).
    Lighter,
}

impl DeviceType {
    /// Feature types (Connect API `type` values) a device of this archetype must provide.
    pub fn required_features(self) -> &'static [&'static str] {
        use DeviceType::*;
        match self {
            Battery | Vehicle => &["batteryState"],
            ContactSensor => &["contactSensorState"],
            EnergyMeter | EvCharger | GridConnectionPoint | Hvac | Inverter => &["currentPower"],
            HeatPump => &["climateOperationMode"],
            Light | Switch => &["switchState"],
            OccupancySensor => &["presenceDetected"],
            SmokeAlarm => &["smokeAlarm"],
            Thermostat => &["setPointTemperature"],
            WaterSensor => &["waterlevelDetected"],
            WindowCovering => &["shutterLevel"],
            ClimateSensor | ParticulateMatterSensor | SwitchInput => &[],
        }
    }

    /// Feature types (Connect API `type` values) a device of this archetype may additionally provide.
    pub fn optional_features(self) -> &'static [&'static str] {
        use DeviceType::*;
        match self {
            Battery => &["maintenance", "currentPower", "energyCounter"],
            ClimateSensor => &[
                "storm",
                "sunshineDuration",
                "humidity",
                "actualTemperature",
                "illumination",
                "windSpeed",
                "sunshine",
                "maintenance",
                "windDirection",
                "co2",
                "raining",
                "rainCount",
            ],
            ContactSensor | OccupancySensor | SmokeAlarm | SwitchInput => &["maintenance"],
            EnergyMeter | EvCharger | GridConnectionPoint | Hvac | Inverter => {
                &["maintenance", "energyCounter"]
            }
            HeatPump => &[
                "coolingTemperatureOffset",
                "presenceMode",
                "hotWaterBoost",
                "heatingTemperatureOffset",
                "maintenance",
                "supplyTemperature",
            ],
            Light => &[
                "colorTemperature",
                "color",
                "onTime",
                "dimming",
                "maintenance",
            ],
            ParticulateMatterSensor => &[
                "particulateMassOne",
                "actualTemperature",
                "particulateMassTwoPointFive",
                "humidity",
                "particulateTypicalSize",
                "maintenance",
                "particulateMassTen",
            ],
            Switch => &["onTime", "maintenance"],
            Thermostat => &["humidity", "actualTemperature", "maintenance", "co2"],
            Vehicle => &["maintenance", "vehicleRange"],
            WaterSensor => &["moistureDetected", "maintenance"],
            WindowCovering => &["slatsLevel", "shutterDirection", "maintenance"],
        }
    }
}
