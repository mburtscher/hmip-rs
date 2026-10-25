use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::types::{
    AckType, BehaviorType, ConfigUpdateResponseStatus, MessageCategory, PluginReadinessStatus,
};
use crate::domain::{Device, ErrorInfo, Feature, GroupTemplate, PropertyTemplate};

macro_rules! empty_bodies {
    ($($(#[$meta:meta])* $name:ident,)*) => {
        $(
            $(#[$meta])*
            #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name {}
        )*
    };
}

empty_bodies! {
    /// Body of `DISCOVER_REQUEST`; carries no data.
    DiscoverRequestBody,
    /// Body of `LIST_USER_MESSAGES_REQUEST`; carries no data.
    ListUserMessagesRequestBody,
    /// Body of `PLUGIN_STATE_REQUEST`; carries no data.
    PluginStateRequestBody,
    /// Body of `SYSTEM_INFO_REQUEST`; carries no data.
    SystemInfoRequestBody,
}

/// Body of `CONFIG_TEMPLATE_REQUEST`: the HCU asks for the plugin's configuration template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigTemplateRequestBody {
    /// ISO 639-1 language the template shall be returned in; `en` and `de` are required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}

/// Body of `CONFIG_TEMPLATE_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigTemplateResponseBody {
    /// Group identifier to template mapping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<HashMap<String, GroupTemplate>>,
    /// Property identifier to template mapping.
    pub properties: HashMap<String, PropertyTemplate>,
}

/// Body of `CONFIG_UPDATE_REQUEST`: a user saved a new plugin configuration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigUpdateRequestBody {
    /// Language for the feedback message in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    /// Property identifier to value mapping. Values are strings, numbers or booleans depending on
    /// the property's [`PropertyType`](crate::domain::types::PropertyType).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
}

/// Body of `CONFIG_UPDATE_RESPONSE`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateResponseBody {
    /// Result or intermediate status of the update. May be sent several times for multi-step updates.
    pub status: ConfigUpdateResponseStatus,
    /// Feedback shown in HCUweb, in the requested language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Default for ConfigUpdateResponseBody {
    fn default() -> Self {
        Self {
            status: ConfigUpdateResponseStatus::Applied,
            message: None,
        }
    }
}

/// Body of `CONTROL_REQUEST`: set target values of a plugin device's features.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ControlRequestBody {
    /// Unique identifier of the device.
    pub device_id: String,
    /// Features with target values; features not included stay unchanged.
    pub features: Vec<Feature>,
}

/// Body of `CONTROL_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ControlResponseBody {
    /// Unique identifier of the device.
    pub device_id: String,
    /// Whether the request was processed successfully.
    pub success: bool,
    /// Details about the failure, if the request did not succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Body of `CREATE_USER_MESSAGE_REQUEST`: show a message in the Homematic IP app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserMessageRequestBody {
    /// How the user can dismiss or acknowledge the message in the app.
    pub behavior_type: BehaviorType,
    /// Localized detailed message, keyed by ISO 639-1 language code.
    pub message: HashMap<String, String>,
    /// Category of the message; determines color and icon in the app.
    pub message_category: MessageCategory,
    /// Time the message was issued.
    pub timestamp: i64,
    /// Localized title, keyed by ISO 639-1 language code.
    pub title: HashMap<String, String>,
    /// An existing message with the same id is replaced.
    pub user_message_id: String,
}

/// Body of `CREATE_USER_MESSAGE_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateUserMessageResponseBody {
    /// Whether the request was processed successfully.
    pub success: bool,
    /// Identifier of the user message.
    pub user_message_id: String,
    /// Details about the failure, if the request did not succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Body of `DELETE_USER_MESSAGE_REQUEST`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeleteUserMessageRequestBody {
    /// Identifier of the user message.
    pub user_message_id: String,
}

/// Body of `DELETE_USER_MESSAGE_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeleteUserMessageResponseBody {
    /// Whether the request was processed successfully.
    pub success: bool,
    /// Identifier of the user message.
    pub user_message_id: String,
    /// Details about the failure, if the request did not succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Body of `DISCOVER_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiscoverResponseBody {
    /// Whether the request was processed successfully.
    pub success: bool,
    /// The devices with their current state.
    pub devices: Vec<Device>,
    /// Details about the failure, if the request did not succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Body of `ERROR_RESPONSE`: the HCU could not process a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponseBody {
    /// Details about the failure, if the request did not succeed.
    pub error: ErrorInfo,
    /// The message that caused the error (not necessarily JSON).
    #[serde(default)]
    pub original_message: Value,
}

/// Body of `EXCLUSION_EVENT`: devices were excluded; stop reporting their state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExclusionEventBody {
    /// Unique identifiers of the devices.
    pub device_ids: Vec<String>,
}

/// Body of `HMIP_SYSTEM_EVENT`: partial update of the Homematic IP system state.
///
/// The transaction object is passed through untyped; see the *Homematic IP system events*
/// chapter of the Connect API documentation for its structure.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HmipSystemEventBody {
    /// Transaction object containing the push events.
    pub event_transaction: Value,
}

/// Body of `HMIP_SYSTEM_REQUEST`: query or control the Homematic IP system.
///
/// `body` is structured according to the schema of `path` (an empty object if it has none).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HmipSystemRequestBody {
    /// Request path, e.g. `/hmip/device/control/setSwitchState`.
    pub path: String,
    /// Request body, structured according to the schema of `path`; an empty object if it has none.
    #[serde(default = "empty_object")]
    pub body: Value,
}

impl HmipSystemRequestBody {
    /// A request for `path` with the given body.
    pub fn new(path: impl Into<String>, body: Value) -> Self {
        Self {
            path: path.into(),
            body,
        }
    }
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

/// Body of `HMIP_SYSTEM_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HmipSystemResponseBody {
    /// HTTP-like status code; anything but 200 usually comes with an error body.
    pub code: i32,
    /// Optional response body; for non-200 codes typically an error with a detailed error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// Body of `INCLUSION_EVENT`: the devices currently included in the HCU. Answer with a
/// `STATUS_RESPONSE` for all of them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InclusionEventBody {
    /// Unique identifiers of the devices.
    pub device_ids: Vec<String>,
}

/// Body of `LIST_USER_MESSAGES_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListUserMessagesResponseBody {
    /// User message id to creation timestamp.
    pub user_message_map: HashMap<String, i64>,
}

/// Body of `PLUGIN_STATE_RESPONSE`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStateResponseBody {
    /// The plugin state shown in HCUweb.
    pub plugin_readiness_status: PluginReadinessStatus,
    /// Localized plugin name keyed by ISO 639-1 language code; the key `de` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<HashMap<String, String>>,
}

impl PluginStateResponseBody {
    /// A state response without friendly names.
    pub fn new(plugin_readiness_status: PluginReadinessStatus) -> Self {
        Self {
            plugin_readiness_status,
            friendly_name: None,
        }
    }
}

/// Body of `STATUS_EVENT`: partial state update of a single device.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusEventBody {
    /// Unique identifier of the device.
    pub device_id: String,
    /// Updated features; features not included are unchanged.
    pub features: Vec<Feature>,
}

/// Body of `STATUS_REQUEST`: the HCU wants the current state of some or all devices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusRequestBody {
    /// Devices to report; `None` means all devices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_ids: Option<Vec<String>>,
}

/// Body of `STATUS_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatusResponseBody {
    /// Whether the request was processed successfully.
    pub success: bool,
    /// The devices with their current state.
    pub devices: Vec<Device>,
    /// Details about the failure, if the request did not succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

/// Body of `SYSTEM_INFO_RESPONSE`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemInfoResponseBody {
    /// IP address of the Home Control Unit.
    pub ip_address: String,
    /// `false` if the HCU is in offline mode.
    pub is_online_mode: bool,
    /// Whether the request was processed successfully.
    pub success: bool,
}

/// Body of `USER_MESSAGE_ACK_EVENT`: a user acknowledged a message in the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageAckEventBody {
    /// How the user acknowledged the message.
    pub ack_type: AckType,
    /// Identifier of the user message.
    pub user_message_id: String,
}
