use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::bodies::*;
use crate::error::{Error, Result};

macro_rules! payloads {
    ($($variant:ident($body:ident) => $wire:literal, $doc:literal;)*) => {
        /// The typed `type` + `body` part of a [`Message`].
///
/// Serialized as the adjacent pair `{"type": "<WIRE_NAME>", "body": {...}}`. Every body struct
/// converts into its variant with `From`, so `Message::new(body)` and `Context::send(body)` work
/// without naming the variant. Variants document the direction (HCU → plugin, plugin → HCU) of
/// the message.
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "type", content = "body")]
        #[non_exhaustive]
        pub enum Payload {
            $(
                #[doc = $doc]
                #[serde(rename = $wire)]
                $variant($body),
            )*
        }

        impl Payload {
            /// The wire name of the message type, e.g. `DISCOVER_REQUEST`.
            pub fn type_name(&self) -> &'static str {
                match self {
                    $(Payload::$variant(_) => $wire,)*
                }
            }

            fn is_known_type(name: &str) -> bool {
                matches!(name, $($wire)|*)
            }
        }

        $(
            impl From<$body> for Payload {
                fn from(body: $body) -> Self {
                    Payload::$variant(body)
                }
            }
        )*
    };
}

payloads! {
    ConfigTemplateRequest(ConfigTemplateRequestBody) => "CONFIG_TEMPLATE_REQUEST", "HCU → plugin: asks for the configuration template shown in HCUweb. Answer with `CONFIG_TEMPLATE_RESPONSE`.";
    ConfigTemplateResponse(ConfigTemplateResponseBody) => "CONFIG_TEMPLATE_RESPONSE", "plugin → HCU: the configuration template (groups and properties with constraints).";
    ConfigUpdateRequest(ConfigUpdateRequestBody) => "CONFIG_UPDATE_REQUEST", "HCU → plugin: a user saved a new configuration. Answer with `CONFIG_UPDATE_RESPONSE`.";
    ConfigUpdateResponse(ConfigUpdateResponseBody) => "CONFIG_UPDATE_RESPONSE", "plugin → HCU: result (or intermediate status) of applying a configuration update.";
    ControlRequest(ControlRequestBody) => "CONTROL_REQUEST", "HCU → plugin: set target values on a plugin device. Answer with `CONTROL_RESPONSE`.";
    ControlResponse(ControlResponseBody) => "CONTROL_RESPONSE", "plugin → HCU: result of a control request.";
    CreateUserMessageRequest(CreateUserMessageRequestBody) => "CREATE_USER_MESSAGE_REQUEST", "plugin → HCU: show a message in the Homematic IP smartphone app.";
    CreateUserMessageResponse(CreateUserMessageResponseBody) => "CREATE_USER_MESSAGE_RESPONSE", "HCU → plugin: result of creating a user message.";
    DeleteUserMessageRequest(DeleteUserMessageRequestBody) => "DELETE_USER_MESSAGE_REQUEST", "plugin → HCU: delete a user message created by the plugin.";
    DeleteUserMessageResponse(DeleteUserMessageResponseBody) => "DELETE_USER_MESSAGE_RESPONSE", "HCU → plugin: result of deleting a user message.";
    DiscoverRequest(DiscoverRequestBody) => "DISCOVER_REQUEST", "HCU → plugin: asks which devices are available for inclusion. Answer with `DISCOVER_RESPONSE`.";
    DiscoverResponse(DiscoverResponseBody) => "DISCOVER_RESPONSE", "plugin → HCU: the discovered devices.";
    ErrorResponse(ErrorResponseBody) => "ERROR_RESPONSE", "HCU → plugin: a message of the plugin could not be processed.";
    ExclusionEvent(ExclusionEventBody) => "EXCLUSION_EVENT", "HCU → plugin: devices were excluded; stop reporting their state.";
    HmipSystemEvent(HmipSystemEventBody) => "HMIP_SYSTEM_EVENT", "HCU → plugin: partial update of the Homematic IP system state (needs the `hmip-system-events` header).";
    HmipSystemRequest(HmipSystemRequestBody) => "HMIP_SYSTEM_REQUEST", "plugin → HCU: query or control the Homematic IP system.";
    HmipSystemResponse(HmipSystemResponseBody) => "HMIP_SYSTEM_RESPONSE", "HCU → plugin: result of a system request.";
    InclusionEvent(InclusionEventBody) => "INCLUSION_EVENT", "HCU → plugin: the devices currently included in the system. Answer with a `STATUS_RESPONSE`.";
    ListUserMessagesRequest(ListUserMessagesRequestBody) => "LIST_USER_MESSAGES_REQUEST", "plugin → HCU: list the user messages created by the plugin.";
    ListUserMessagesResponse(ListUserMessagesResponseBody) => "LIST_USER_MESSAGES_RESPONSE", "HCU → plugin: the user messages created by the plugin.";
    PluginStateRequest(PluginStateRequestBody) => "PLUGIN_STATE_REQUEST", "HCU → plugin: asks for the plugin state. Answer with `PLUGIN_STATE_RESPONSE`.";
    PluginStateResponse(PluginStateResponseBody) => "PLUGIN_STATE_RESPONSE", "plugin → HCU: the plugin's readiness and name; sent on startup, on request and whenever it changes.";
    StatusEvent(StatusEventBody) => "STATUS_EVENT", "plugin → HCU: partial state update of a single device.";
    StatusRequest(StatusRequestBody) => "STATUS_REQUEST", "HCU → plugin: asks for the state of some or all devices. Answer with `STATUS_RESPONSE`.";
    StatusResponse(StatusResponseBody) => "STATUS_RESPONSE", "plugin → HCU: full state of devices; may also be sent unsolicited.";
    SystemInfoRequest(SystemInfoRequestBody) => "SYSTEM_INFO_REQUEST", "plugin → HCU: asks for system information such as the HCU's IP address.";
    SystemInfoResponse(SystemInfoResponseBody) => "SYSTEM_INFO_RESPONSE", "HCU → plugin: system information.";
    UserMessageAckEvent(UserMessageAckEventBody) => "USER_MESSAGE_ACK_EVENT", "HCU → plugin: a user acknowledged a message in the app.";
}

/// A complete websocket message: envelope plus [`Payload`].
///
/// ```
/// use hmip_rs::api::{Message, PluginStateResponseBody};
/// use hmip_rs::domain::types::PluginReadinessStatus;
///
/// let message = Message::new(PluginStateResponseBody::new(PluginReadinessStatus::Ready))
///     .with_id("38967997-e1b3-463f-8dc4-f889bb5d10a2");
/// assert_eq!(
///     message.to_json().unwrap(),
///     r#"{"id":"38967997-e1b3-463f-8dc4-f889bb5d10a2","type":"PLUGIN_STATE_RESPONSE","body":{"pluginReadinessStatus":"READY"}}"#
/// );
/// ```
///
/// `id` and `plugin_id` are filled in by the [`Plugin`](crate::Plugin) when left unset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Message identifier; responses reuse the id of their request.
    pub id: Option<String>,
    /// Unique identifier of the plugin; filled in when sending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
    /// Message type and body.
    #[serde(flatten)]
    pub payload: Payload,
}

impl Message {
    /// Wraps a payload (or any body type convertible to one) in an envelope without ids.
    pub fn new(payload: impl Into<Payload>) -> Self {
        Self {
            id: None,
            plugin_id: None,
            payload: payload.into(),
        }
    }

    /// Sets the message id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Parses a message from JSON text.
    ///
    /// A missing `body` is treated as an empty one, and an unrecognised `type` yields
    /// [`Error::UnknownMessageType`] so callers can tell it apart from malformed JSON.
    pub fn from_json(text: &str) -> Result<Self> {
        let mut value: Value = serde_json::from_str(text)?;
        if let Some(object) = value.as_object_mut() {
            if let Some(Value::String(ty)) = object.get("type") {
                if !Payload::is_known_type(ty) {
                    return Err(Error::UnknownMessageType(ty.clone()));
                }
            }
            object
                .entry("body")
                .or_insert_with(|| Value::Object(Default::default()));
        }
        Ok(serde_json::from_value(value)?)
    }

    /// Serializes the message to JSON text.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
}

impl<P: Into<Payload>> From<P> for Message {
    fn from(payload: P) -> Self {
        Message::new(payload)
    }
}
