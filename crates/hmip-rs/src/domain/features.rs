//! Device features.
//!
//! Every feature is a plain struct whose fields are all optional, mirroring the Connect API where
//! a feature only reports the values it knows about. The [`Feature`] enum is the tagged union that
//! is sent over the wire (`{"type": "actualTemperature", ...}`).
//!
//! Value ranges documented by the Connect API are checked by `validate()` on each feature, by
//! [`Feature::validate`] and by [`Device::validate`](super::Device::validate). Deserialization is
//! deliberately lenient so that out-of-range values sent by the Home Control Unit never break
//! message handling.
//!
//! ```
//! use hmip_rs::domain::Feature;
//! use hmip_rs::domain::features::{BatteryStateFeature, DimmingFeature};
//!
//! // Build a feature; every field is optional.
//! let battery = BatteryStateFeature { battery_level: Some(0.8), ..Default::default() };
//! assert!(battery.validate().is_ok());
//! assert!(BatteryStateFeature { battery_level: Some(1.5), ..Default::default() }.validate().is_err());
//!
//! // On the wire it is tagged with its type.
//! let feature: Feature = DimmingFeature { dim_level: Some(0.2) }.into();
//! assert_eq!(serde_json::to_string(&feature).unwrap(), r#"{"type":"dimming","dimLevel":0.2}"#);
//! assert_eq!(feature.type_name(), "dimming");
//! ```
//!
//! Special values of the API (for example `1.005` = "last value" and `1.01` = "ignore" for
//! dimming, shutter and slats levels) are part of the documented ranges and pass validation where
//! the API allows them.

use serde::{Deserialize, Serialize};

use super::types::{ClimateOperationType, PresenceType, ShadingDirection};
use crate::error::{Error, Result};

const INF: f64 = f64::INFINITY;

fn check(feature: &'static str, field: &'static str, value: f64, min: f64, max: f64) -> Result<()> {
    if value < min || value > max {
        return Err(Error::OutOfRange {
            feature,
            field,
            value,
            min,
            max,
        });
    }
    Ok(())
}

macro_rules! features {
    ($(
        $(#[$meta:meta])*
        $variant:ident($name:ident) = $wire:literal {
            $(
                $(#[$fmeta:meta])*
                $field:ident : $ty:ty $(=> $lo:expr, $hi:expr)?
            ),* $(,)?
        }
    )*) => {
        $(
            $(#[$meta])*
            #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
            #[serde(rename_all = "camelCase", default)]
            pub struct $name {
                $(
                    $(#[$fmeta])*
                    #[serde(skip_serializing_if = "Option::is_none")]
                    pub $field: Option<$ty>,
                )*
            }

            impl $name {
                /// Checks the documented value ranges of this feature.
                pub fn validate(&self) -> Result<()> {
                    $($(
                        if let Some(value) = self.$field {
                            check(stringify!($name), stringify!($field), value as f64, $lo, $hi)?;
                        }
                    )?)*
                    Ok(())
                }
            }

            impl From<$name> for Feature {
                fn from(feature: $name) -> Self {
                    Feature::$variant(feature)
                }
            }
        )*

        /// A single device feature, tagged with its Connect API `type`.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase")]
        #[non_exhaustive]
        pub enum Feature {
            $(
                $(#[$meta])*
                $variant($name),
            )*
        }

        impl Feature {
            /// The Connect API `type` of the wrapped feature, e.g. `actualTemperature`.
            pub fn type_name(&self) -> &'static str {
                match self {
                    $(Feature::$variant(_) => $wire,)*
                }
            }

            /// Checks the documented value ranges of the wrapped feature.
            pub fn validate(&self) -> Result<()> {
                match self {
                    $(Feature::$variant(feature) => feature.validate(),)*
                }
            }
        }
    };
}

features! {
    /// Feature representing the actual temperature measured by a plugin device in °C.
    ActualTemperature(ActualTemperatureFeature) = "actualTemperature" {
        /// The current temperature in °C.
        actual_temperature: f64 => -50.0, 60.0,
    }

    /// Feature representing the battery state of a plugin device.
    BatteryState(BatteryStateFeature) = "batteryState" {
        /// The current state of charge of a battery. From 0 to 1 representing the percentage
        battery_level: f64 => 0.0, 1.0,
        /// The capacity of batter in Wh. Ideally the usable capacity, otherwise the nominal capacity.
        battery_capacity: f64 => 0.0, INF,
    }

    /// Feature representing the climate operation mode. Used mainly by AC.
    ClimateOperationMode(ClimateOperationModeFeature) = "climateOperationMode" {
        /// The operation mode of the device.
        climate_operation_mode: ClimateOperationType,
    }

    /// Feature representing carbon dioxide concentration measured by a plugin device.
    Co2(Co2ConcentrationFeature) = "co2" {
        /// The carbon dioxide concentration in ppm.
        co2: f64 => 0.0, INF,
    }

    /// Feature representing the light color of a plugin device.
    Color(ColorFeature) = "color" {
        /// The hue color value in degrees on the color wheel. Special values are 360 for the last value and 361 as the ignore value.
        hue: i32 => 0.0, 361.0,
        /// The color intensity. Special values are 1.005 for the last value and 1.01 as the ignore value.
        saturation_level: f64 => 0.0, 1.0,
    }

    /// Feature representing the color temperature of a plugin device.
    ColorTemperature(ColorTemperatureFeature) = "colorTemperature" {
        /// The current color temperature in Kelvin.
        color_temperature: i32 => 0.0, INF,
        /// The maximum color temperature supported by the device in Kelvin.
        maximum_color_temperature: i32 => 0.0, INF,
        /// The minimal color temperature supported by the device in Kelvin.
        minimal_color_temperature: i32 => 0.0, INF,
    }

    /// Feature representing the contact sensor state of a plugin device.
    ContactSensorState(ContactSensorStateFeature) = "contactSensorState" {
        /// Flag to indicate whether the contact sensor is triggered.
        triggered: bool,
    }

    /// Feature representing the cooling offset temperature of a plugin device.
    CoolingTemperatureOffset(CoolingTemperatureOffsetFeature) = "coolingTemperatureOffset" {
        /// The cooling temperature offset in °C.
        cooling_temperature_offset: f64 => -50.0, 50.0,
    }

    /// Feature representing the currently used or produced power of a plugin device.
    CurrentPower(CurrentPowerFeature) = "currentPower" {
        /// The currently consumed or produced power in W. May be negative or positive, depending on the specific device.
        current_power: f64,
    }

    /// Feature representing dimming value of a plugin device.
    Dimming(DimmingFeature) = "dimming" {
        /// The target brightness level. 1.005 is the special value for last value, if supported. 1.01 is the special value for ignore value, if supported.
        dim_level: f64 => 0.0, 1.0,
    }

    /// Feature representing the energy counter of a plugin device.
    EnergyCounter(EnergyCounterFeature) = "energyCounter" {
        /// The total amount of energy in kWh that has flown from the home to the device, e.g. the energy put into a battery since resetting the counter.
        #[serde(rename = "in")]
        energy_in: f64 => 0.0, INF,
        /// The total amount of energy in kWh that has flown from the device to the home, e.g. the energy taken from a battery since resetting the counter.
        #[serde(rename = "out")]
        energy_out: f64 => 0.0, INF,
    }

    /// Feature representing heating offset of a plugin device.
    HeatingTemperatureOffset(HeatingTemperatureOffsetFeature) = "heatingTemperatureOffset" {
        /// The heating temperature offset in °C.
        heating_temperature_offset: f64 => -50.0, 50.0,
    }

    /// Feature representing the hot water boost function of a plugin device. Mainly used by heat pumps.
    HotWaterBoost(HotWaterBoostFeature) = "hotWaterBoost" {
        /// Flag to indicate whether the hot water boost function is active or not.
        on: bool,
    }

    /// Feature representing the humidity value measured by a plugin device.
    Humidity(HumidityFeature) = "humidity" {
        /// The current humidity in %rh.
        humidity: i32 => 0.0, 100.0,
    }

    /// Feature representing the current illumination measured by a plugin device.
    Illumination(IlluminationFeature) = "illumination" {
        /// The current illumination in lux.
        illumination: f64 => 0.0, 20000.0,
    }

    /// Feature representing general maintenance functions of a plugin device.
    Maintenance(MaintenanceFeature) = "maintenance" {
        /// Flag to indicate whether the device's battery is low.
        low_bat: bool,
        /// Flag to indicate whether the sabotage contact sensor of the device is triggered.
        sabotage: bool,
        /// Flag to indicate whether the device is currently unreachable.
        unreach: bool,
    }

    /// Feature representing the moisture detection function of a plugin device.
    MoistureDetected(MoistureDetectedFeature) = "moistureDetected" {
        /// Flag to indicate whether the device has recently detected moisture.
        moisture_detected: bool,
    }

    /// Feature representing the time for which a plugin device shall remain switched on. Intended to be included in a control request to automatically turn off the device after the desired time.
    OnTime(OnTimeFeature) = "onTime" {
        /// Desired time in seconds for which the device shall remain switched on.
        on_time: f64 => 0.0, INF,
    }

    /// Feature representing the mass concentration of particulate matter of size PM 1 measured by a plugin device.
    ParticulateMassOne(ParticulateMassOneFeature) = "particulateMassOne" {
        /// The mass concentration of particulate matter of size PM 1 in µg/m3.
        particulate_mass_concentration_one: f64 => 0.0, INF,
        /// The average mass concentration of particulate matter of size PM 1 in µg/m3 over 24 h.
        particulate_mass_concentration_one_average: f64 => 0.0, INF,
        /// The number concentration of particulate matter of size PM 1 µg/cm3.
        particulate_number_concentration_one: f64 => 0.0, INF,
    }

    /// Feature representing the mass concentration of particulate matter of size PM 10 measured by a plugin device.
    ParticulateMassTen(ParticulateMassTenFeature) = "particulateMassTen" {
        /// The air quality index of particulate matter of size PM 10
        air_quality_index_ten: f64 => 0.0, INF,
        /// The mass concentration of particulate matter of size PM 10 in µg/m3.
        particulate_mass_concentration_ten: f64 => 0.0, INF,
        /// The average mass concentration of particulate matter of size PM 10 in µg/m3 over 24 h.
        particulate_mass_concentration_ten_average: f64 => 0.0, INF,
        /// The number concentration of particulate matter of size PM 10 µg/cm3.
        particulate_number_concentration_ten: f64 => 0.0, INF,
        /// The number concentration of particulate matter of size PM 10 in µg/m3 over 24 h.
        particulate_number_concentration_ten_average: f64 => 0.0, INF,
    }

    /// Feature representing the mass concentration of particulate matter of size PM 2.5 measured by a plugin device.
    ParticulateMassTwoPointFive(ParticulateMassTwoPointFiveFeature) = "particulateMassTwoPointFive" {
        /// The air quality index of particulate matter of size PM 2.5
        air_quality_index_two_point_five: f64 => 0.0, INF,
        /// The mass concentration of particulate matter of size PM 2.5 in µg/m3.
        particulate_mass_concentration_two_point_five: f64 => 0.0, INF,
        /// The average mass concentration of particulate matter of size PM 2.5 in µg/m3 over 24 h.
        particulate_mass_concentration_two_point_five_average: f64 => 0.0, INF,
        /// The number concentration of particulate matter of size PM 2.5 µg/cm3.
        particulate_number_concentration_two_point_five: f64 => 0.0, INF,
        /// The number concentration of particulate matter of size PM 2.5 in µg/m3 over 24 h.
        particulate_number_concentration_two_point_five_average: f64 => 0.0, INF,
    }

    /// Feature representing typical particle size measured by a plugin device.
    ParticulateTypicalSize(ParticulateTypicalSizeFeature) = "particulateTypicalSize" {
        /// The typical particle size in 0,01 µm resolution.
        particulate_typical_size: f64 => 0.0, INF,
    }

    /// Feature representing the presence detection function of a plugin device.
    PresenceDetected(PresenceDetectedFeature) = "presenceDetected" {
        /// Flag to indicate whether the device has recently detected presence.
        presence_detected: bool,
    }

    /// Feature representing the presence mode of a plugin device.
    PresenceMode(PresenceModeFeature) = "presenceMode" {
        /// The currently active presence mode.
        presence_mode: PresenceType,
    }

    /// Feature representing the amount of rain measured by a plugin device.
    RainCount(RainCountFeature) = "rainCount" {
        /// The total accumulated amount of rain in mm.
        rain_counter: f64 => 0.0, INF,
        /// The accumulated amount of today's rain in mm.
        today_rain_counter: f64 => 0.0, INF,
        /// The accumulated amount of yesterday's rain in mm.
        yesterday_rain_counter: f64 => 0.0, INF,
    }

    /// Feature representing the rain sensor of a plugin device.
    Raining(RainingFeature) = "raining" {
        /// Flag to indicate whether it is currently raining.
        raining: bool,
    }

    /// Feature representing the target temperature set in a plugin device.
    SetPointTemperature(SetPointTemperatureFeature) = "setPointTemperature" {
        /// The target temperature in °C the device is currently set to.
        set_point_temperature: f64 => -50.0, 60.0,
    }

    /// Feature representing the shutter direction of a plugin device.
    ShutterDirection(ShutterDirectionFeature) = "shutterDirection" {
        /// The direction in which the shutter moved last.
        shutter_direction: ShadingDirection,
    }

    /// Feature representing the shutter level of a plugin device.
    ShutterLevel(ShutterLevelFeature) = "shutterLevel" {
        /// The current shutter level of the device in a range from 0 - 1. 1 is fully closed. Possible special values are 1.005 for the last value and 1.01 as the ignore value.
        shutter_level: f64 => 0.0, 1.01,
    }

    /// Feature representing the slats level of a plugin device.
    SlatsLevel(SlatsLevelFeature) = "slatsLevel" {
        /// The slats level of the device in a range from 0 - 1. 1 is fully closed. Possible special values are 1.005 for the last value and 1.01 as the ignore value.
        slats_level: f64 => 0.0, 1.01,
    }

    /// Feature representing smoke alarm function of a plugin device.
    SmokeAlarm(SmokeAlarmFeature) = "smokeAlarm" {
        /// Flag to indicate whether the device is currently detecting smoke.
        smoke_alarm: bool,
    }

    /// Feature representing the wind speed and direction measured by a plugin device.
    Storm(StormFeature) = "storm" {
        /// Flag to indicate whether the device currently detects storm.
        storm: bool,
    }

    /// Feature representing sunshine measured by a plugin device.
    Sunshine(SunshineFeature) = "sunshine" {
        /// Flag to indicate whether the device currently detects sunshine.
        sunshine: bool,
    }

    /// Feature representing the sunshine duration measured by a plugin device.
    SunshineDuration(SunshineDurationFeature) = "sunshineDuration" {
        /// The total accumulated duration of sunshine in h.
        sunshine_duration: i32 => 0.0, INF,
        /// The accumulated duration of today's sunshine in h.
        today_sunshine_duration: i32 => 0.0, INF,
        /// The accumulated duration of yesterday's sunshine in h.
        yesterday_sunshine_duration: i32 => 0.0, INF,
    }

    /// Feature representing the supply temperature of a plugin device.
    SupplyTemperature(SupplyTemperatureFeature) = "supplyTemperature" {
        /// The current supply temperature in °C.
        supply_temperature: f64 => -50.0, 60.0,
    }

    /// Feature representing the switch state of a plugin device.
    SwitchState(SwitchStateFeature) = "switchState" {
        /// Flag to indicate whether the device is switched on or off.
        on: bool,
    }

    /// Feature representing the vehicle range of a plugin device.
    VehicleRange(VehicleRangeFeature) = "vehicleRange" {
        /// The remaining travel range of the vehicle in km.
        travel_range: f64 => 0.0, INF,
    }

    /// Feature representing the water level detection function of a plugin device.
    WaterlevelDetected(WaterlevelDetectedFeature) = "waterlevelDetected" {
        /// Flag to indicate whether the device has recently detected a water level.
        waterlevel_detected: bool,
    }

    /// Feature representing the wind direction measured by a plugin device.
    WindDirection(WindDirectionFeature) = "windDirection" {
        /// The current wind direction in °.
        wind_direction: f64 => 0.0, 359.0,
    }

    /// Feature representing the wind speed measured by a plugin device.
    WindSpeed(WindSpeedFeature) = "windSpeed" {
        /// The current wind speed in km/h.
        wind_speed: f64 => 0.0, 400.0,
    }
}
