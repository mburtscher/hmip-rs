//! Message types of the Connect API websocket protocol.
//!
//! Every message on the wire is a JSON object shaped like
//!
//! ```json
//! { "id": "…", "pluginId": "…", "type": "DISCOVER_REQUEST", "body": { … } }
//! ```
//!
//! * [`Message`] models the envelope (`id`, `pluginId`) and carries a [`Payload`].
//! * [`Payload`] is the tagged union of all message types (`type` + `body`). Each variant wraps
//!   one of the body structs in this module, named after the message: `DISCOVER_REQUEST` has
//!   [`DiscoverRequestBody`], `STATUS_RESPONSE` has [`StatusResponseBody`], and so on.
//!
//! Body structs convert into [`Payload`] and [`Message`] with `From`, so most code never names the
//! enum variants:
//!
//! ```
//! use hmip_rs::api::{Message, Payload, StatusRequestBody};
//!
//! let message: Message = StatusRequestBody { device_ids: None }.into();
//! assert!(matches!(message.payload, Payload::StatusRequest(_)));
//! assert_eq!(message.payload.type_name(), "STATUS_REQUEST");
//! ```
//!
//! Fields documented as optional by the Connect API are `Option`s and omitted when serializing;
//! deserialization fills missing fields with defaults, so messages from newer or older HCU
//! versions keep parsing.

mod bodies;
mod message;

pub use bodies::*;
pub use message::{Message, Payload};
