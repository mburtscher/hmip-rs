//! Error type of the crate.

/// Result alias using the crate's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Everything that can go wrong while talking to the Home Control Unit.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No token was configured and none could be read from the token file.
    #[error("token not provided and no token available under {path}")]
    MissingToken {
        /// Location that was probed for a token.
        path: String,
    },

    /// The token cannot be sent as an HTTP header value.
    #[error("token contains characters that are not allowed in a header")]
    InvalidToken,

    /// The plugin was used while no connection is established.
    #[error("plugin is not connected")]
    NotConnected,

    /// A feature value is outside of the range documented by the Connect API.
    #[error("{feature}.{field} = {value} is out of range [{min}, {max}]")]
    OutOfRange {
        /// Feature type name.
        feature: &'static str,
        /// Field name.
        field: &'static str,
        /// Offending value.
        value: f64,
        /// Lowest allowed value.
        min: f64,
        /// Highest allowed value.
        max: f64,
    },

    /// A device uses a feature that is neither required nor optional for its type.
    #[error("feature {feature} is not supported by device type {device_type:?}")]
    FeatureNotSupported {
        /// Device archetype.
        device_type: crate::domain::types::DeviceType,
        /// Offending feature type.
        feature: String,
    },

    /// A device lacks a feature required by its type.
    #[error("device type {device_type:?} requires feature {feature}")]
    MissingFeature {
        /// Device archetype.
        device_type: crate::domain::types::DeviceType,
        /// Missing feature type.
        feature: String,
    },

    /// A message could not be (de)serialized.
    #[error("invalid message: {0}")]
    Json(#[from] serde_json::Error),

    /// A well-formed message carried an unknown `type`.
    #[error("unknown message type: {0}")]
    UnknownMessageType(String),

    /// WebSocket transport failure.
    #[error("websocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),

    /// TLS setup failure.
    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),

    /// I/O failure, e.g. while reading the token file.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(Box::new(err))
    }
}
