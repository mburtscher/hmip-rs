//! The base plugin implementation: connection handling and message dispatch.
//!
//! [`Plugin::run`] connects to the HCU, reports the plugin state and dispatches incoming messages
//! to a [`Handler`]. Handlers answer through the [`Context`] they are given.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{Connector, connect_async_tls_with_config};
use uuid::Uuid;

use crate::api::*;
use crate::domain::Device;
use crate::domain::types::PluginReadinessStatus;
use crate::error::{Error, Result};

/// Host the Home Control Unit is reachable under from inside a plugin container.
pub const DEFAULT_HOST: &str = "host.containers.internal";
/// Port of the Connect API websocket.
pub const DEFAULT_PORT: u16 = 9001;
/// File the HCU mounts the plugin token into.
pub const DEFAULT_TOKEN_PATH: &str = "/TOKEN";

fn unhandled(kind: &str) -> Result<()> {
    tracing::debug!("no handler implemented for {kind}");
    Ok(())
}

macro_rules! handler_methods {
    ($($(#[$meta:meta])* $method:ident($body:ty) => $kind:literal;)*) => {
        $(
            $(#[$meta])*
            fn $method(
                &self,
                ctx: &Context,
                id: Option<String>,
                body: $body,
            ) -> impl Future<Output = Result<()>> + Send {
                let _ = (ctx, id, body);
                async { unhandled($kind) }
            }
        )*
    };
}

/// Reacts to messages sent by the Home Control Unit.
///
/// Every method has a default, so implementors only override what their plugin supports. Each
/// incoming message is handled in its own task; answer through the [`Context`], usually with
/// [`Context::reply`] so the response carries the request's id.
///
/// The defaults follow the Connect API's recommendations: a `PLUGIN_STATE_REQUEST` is answered
/// with the plugin's current readiness (see [`Context::set_readiness`]) and a
/// `CONFIG_TEMPLATE_REQUEST` with an empty template. Everything else is logged and ignored.
pub trait Handler: Send + Sync + 'static {
    /// The HCU asks for the plugin's state. Default: reply with the current readiness.
    fn on_plugin_state_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        body: PluginStateRequestBody,
    ) -> impl Future<Output = Result<()>> + Send {
        let _ = body;
        async move { ctx.reply(id, ctx.state_response()) }
    }

    /// The HCU asks for the configuration template. Default: an empty template.
    fn on_config_template_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        body: ConfigTemplateRequestBody,
    ) -> impl Future<Output = Result<()>> + Send {
        let _ = body;
        async move { ctx.reply(id, ConfigTemplateResponseBody::default()) }
    }

    /// Called once right after the connection is established and the initial state was queued.
    ///
    /// Use it to keep a clone of the [`Context`] for pushing unsolicited messages, e.g. periodic
    /// `STATUS_RESPONSE`s. The returned future may run for as long as it likes; it is aborted when
    /// the connection ends (and `Context::send` fails with [`Error::NotConnected`] by then).
    fn on_connected(&self, ctx: &Context) -> impl Future<Output = Result<()>> + Send {
        let _ = ctx;
        async { Ok(()) }
    }

    handler_methods! {
        /// A user saved a new configuration; answer with a `CONFIG_UPDATE_RESPONSE`.
        on_config_update_request(ConfigUpdateRequestBody) => "CONFIG_UPDATE_REQUEST";
        /// Control a device; answer with a `CONTROL_RESPONSE`.
        on_control_request(ControlRequestBody) => "CONTROL_REQUEST";
        /// The HCU wants to know which devices are available; answer with a `DISCOVER_RESPONSE`
        /// (or ignore it if the plugin includes no devices).
        on_discover_request(DiscoverRequestBody) => "DISCOVER_REQUEST";
        /// Devices were included; answer with a `STATUS_RESPONSE` for all of them.
        on_inclusion_event(InclusionEventBody) => "INCLUSION_EVENT";
        /// Devices were excluded; stop reporting their state.
        on_exclusion_event(ExclusionEventBody) => "EXCLUSION_EVENT";
        /// The HCU wants the current state of some or all devices.
        on_status_request(StatusRequestBody) => "STATUS_REQUEST";
        /// Partial update of the Homematic IP system (needs [`PluginBuilder::hmip_system_events`]).
        on_hmip_system_event(HmipSystemEventBody) => "HMIP_SYSTEM_EVENT";
        /// Result of an `HMIP_SYSTEM_REQUEST`; correlate via the id.
        on_hmip_system_response(HmipSystemResponseBody) => "HMIP_SYSTEM_RESPONSE";
        /// Result of a `SYSTEM_INFO_REQUEST`; correlate via the id.
        on_system_info_response(SystemInfoResponseBody) => "SYSTEM_INFO_RESPONSE";
        /// Result of a `CREATE_USER_MESSAGE_REQUEST`; correlate via the id.
        on_create_user_message_response(CreateUserMessageResponseBody) => "CREATE_USER_MESSAGE_RESPONSE";
        /// Result of a `DELETE_USER_MESSAGE_REQUEST`; correlate via the id.
        on_delete_user_message_response(DeleteUserMessageResponseBody) => "DELETE_USER_MESSAGE_RESPONSE";
        /// Result of a `LIST_USER_MESSAGES_REQUEST`; correlate via the id.
        on_list_user_messages_response(ListUserMessagesResponseBody) => "LIST_USER_MESSAGES_RESPONSE";
        /// A user acknowledged a message in the Homematic IP app.
        on_user_message_ack_event(UserMessageAckEventBody) => "USER_MESSAGE_ACK_EVENT";
        /// The HCU could not process a message the plugin sent.
        on_error_response(ErrorResponseBody) => "ERROR_RESPONSE";
    }
}

/// A handler that only uses the defaults; the plugin then only reports its state and answers
/// state and (empty) configuration template requests.
impl Handler for () {}

#[derive(Debug)]
struct State {
    readiness: PluginReadinessStatus,
    friendly_name: Option<HashMap<String, String>>,
}

/// Cheap, cloneable handle for sending messages to the Home Control Unit.
///
/// Sending only queues the message, so [`Context::send`] and [`Context::reply`] are synchronous.
/// Once the connection is gone they fail with [`Error::NotConnected`](crate::Error::NotConnected),
/// which is how background loops learn that they should stop:
///
/// ```no_run
/// # use hmip_rs::{Context, Handler, Result};
/// # use hmip_rs::api::StatusEventBody;
/// struct Pusher;
///
/// impl Handler for Pusher {
///     async fn on_connected(&self, ctx: &Context) -> Result<()> {
///         loop {
///             tokio::time::sleep(std::time::Duration::from_secs(10)).await;
///             ctx.send(StatusEventBody { device_id: "d1".into(), features: vec![] })?;
///         }
///     }
/// }
/// ```
///
/// Obtained in [`Handler`] callbacks; clone it to push unsolicited messages from other tasks.
#[derive(Debug, Clone)]
pub struct Context {
    plugin_id: Arc<str>,
    outgoing: mpsc::UnboundedSender<WsMessage>,
    state: Arc<Mutex<State>>,
}

impl Context {
    /// The id this plugin identified itself with.
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Sends a message. A missing `plugin_id` and `id` are filled in (unsolicited messages get a
    /// random UUID); device features in discovery and status responses are validated first.
    ///
    /// Returns once the message is queued; transmission happens on the connection task.
    pub fn send(&self, message: impl Into<Message>) -> Result<()> {
        let mut message = message.into();
        match &message.payload {
            Payload::DiscoverResponse(body) => {
                body.devices.iter().try_for_each(Device::validate)?
            }
            Payload::StatusResponse(body) => body.devices.iter().try_for_each(Device::validate)?,
            Payload::StatusEvent(body) => body
                .features
                .iter()
                .try_for_each(crate::domain::Feature::validate)?,
            _ => {}
        }
        if message.plugin_id.as_deref().is_none_or(str::is_empty) {
            message.plugin_id = Some(self.plugin_id.to_string());
        }
        message.id.get_or_insert_with(|| Uuid::new_v4().to_string());

        let json = message.to_json()?;
        tracing::debug!("[SEND] {json}");
        self.outgoing
            .send(WsMessage::text(json))
            .map_err(|_| Error::NotConnected)
    }

    /// Sends `payload` as the answer to the request with the given id.
    pub fn reply(&self, request_id: Option<String>, payload: impl Into<Payload>) -> Result<()> {
        let mut message = Message::new(payload);
        message.id = request_id;
        self.send(message)
    }

    /// The `PLUGIN_STATE_RESPONSE` body describing the current state.
    pub fn state_response(&self) -> PluginStateResponseBody {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        PluginStateResponseBody {
            plugin_readiness_status: state.readiness,
            friendly_name: state.friendly_name.clone(),
        }
    }

    /// Records a new readiness status and reports it to the HCU. Call this whenever it changes,
    /// e.g. from `CONFIG_REQUIRED` to `READY` after a configuration was applied successfully.
    /// The recorded status is also what the default `PLUGIN_STATE_REQUEST` handler answers with.
    pub fn set_readiness(&self, status: PluginReadinessStatus) -> Result<()> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .readiness = status;
        self.send(self.state_response())
    }
}

/// Configures and creates a [`Plugin`].
#[derive(Debug, Clone)]
pub struct PluginBuilder {
    id: String,
    host: String,
    port: u16,
    token: Option<String>,
    token_path: String,
    tls: bool,
    readiness: PluginReadinessStatus,
    friendly_name: Option<HashMap<String, String>>,
    hmip_system_events: bool,
}

impl PluginBuilder {
    /// Host of the HCU. Defaults to [`DEFAULT_HOST`]; remote plugins use `hcu1-XXXX.local`.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Port of the websocket. Defaults to [`DEFAULT_PORT`].
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Authentication token. Defaults to the contents of the token file.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// File to read the token from when none is set. Defaults to [`DEFAULT_TOKEN_PATH`].
    pub fn token_path(mut self, path: impl Into<String>) -> Self {
        self.token_path = path.into();
        self
    }

    /// Use plain `ws://` instead of `wss://`. Only meant for tests against a local server.
    pub fn plaintext(mut self) -> Self {
        self.tls = false;
        self
    }

    /// Readiness reported right after connecting. Defaults to `READY`.
    pub fn initial_readiness(mut self, status: PluginReadinessStatus) -> Self {
        self.readiness = status;
        self
    }

    /// Adds a localized plugin name (ISO 639-1 language code) to the state responses. The
    /// Connect API requires the `de` entry once any name is given.
    pub fn friendly_name(mut self, language: impl Into<String>, name: impl Into<String>) -> Self {
        self.friendly_name
            .get_or_insert_with(HashMap::new)
            .insert(language.into(), name.into());
        self
    }

    /// Subscribe to Homematic IP system events (`hmip-system-events` header), delivered to
    /// [`Handler::on_hmip_system_event`].
    pub fn hmip_system_events(mut self, enabled: bool) -> Self {
        self.hmip_system_events = enabled;
        self
    }

    /// Finishes configuration.
    pub fn build(self) -> Plugin {
        Plugin { config: self }
    }
}

/// A plugin for the HomematicIP Home Control Unit.
///
/// ```no_run
/// use hmip_rs::Plugin;
/// use hmip_rs::domain::types::PluginReadinessStatus;
///
/// # async fn run() -> hmip_rs::Result<()> {
/// // Installed plugin: defaults are right.
/// Plugin::new("com.example.hello").run(()).await?;
///
/// // Remote development against a Home Control Unit in the local network.
/// Plugin::builder("com.example.hello")
///     .host("hcu1-1234.local")
///     .token("…")
///     .initial_readiness(PluginReadinessStatus::ConfigRequired)
///     .friendly_name("de", "Hallo")
///     .friendly_name("en", "Hello")
///     .build()
///     .run(())
///     .await
/// # }
/// ```
///
/// Create it with [`Plugin::new`] or [`Plugin::builder`] and drive it with [`Plugin::run`], which
/// connects, announces readiness and dispatches incoming messages to a [`Handler`] until the
/// connection ends.
#[derive(Debug, Clone)]
pub struct Plugin {
    config: PluginBuilder,
}

impl Plugin {
    /// A plugin with default connection settings (container host, token from `/TOKEN`).
    pub fn new(id: impl Into<String>) -> Self {
        Self::builder(id).build()
    }

    /// Starts configuring a plugin.
    pub fn builder(id: impl Into<String>) -> PluginBuilder {
        PluginBuilder {
            id: id.into(),
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            token: None,
            token_path: DEFAULT_TOKEN_PATH.to_owned(),
            tls: true,
            readiness: PluginReadinessStatus::Ready,
            friendly_name: None,
            hmip_system_events: false,
        }
    }

    /// The plugin id.
    pub fn id(&self) -> &str {
        &self.config.id
    }

    async fn token(&self) -> Result<String> {
        if let Some(token) = &self.config.token {
            return Ok(token.clone());
        }
        match tokio::fs::read_to_string(&self.config.token_path).await {
            Ok(token) => Ok(token.trim().to_owned()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(Error::MissingToken {
                path: self.config.token_path.clone(),
            }),
            Err(err) => Err(err.into()),
        }
    }

    /// Connects to the HCU and serves messages until the connection closes.
    ///
    /// Returns `Ok(())` on a clean close by the HCU. Drop the future (e.g. via `tokio::select!`)
    /// to stop the plugin.
    pub async fn run<H: Handler>(&self, handler: H) -> Result<()> {
        let cfg = &self.config;
        let token = self.token().await?;

        let scheme = if cfg.tls { "wss" } else { "ws" };
        let mut request = format!("{scheme}://{}:{}", cfg.host, cfg.port).into_client_request()?;
        let headers = request.headers_mut();
        headers.insert(
            "authtoken",
            HeaderValue::from_str(&token).map_err(|_| Error::InvalidToken)?,
        );
        headers.insert(
            "plugin-id",
            HeaderValue::from_str(&cfg.id).map_err(|_| Error::InvalidToken)?,
        );
        if cfg.hmip_system_events {
            headers.insert("hmip-system-events", HeaderValue::from_static("true"));
        }

        let connector = if cfg.tls {
            Connector::Rustls(Arc::new(insecure_tls_config()?))
        } else {
            Connector::Plain
        };
        let (socket, _) =
            connect_async_tls_with_config(request, None, false, Some(connector)).await?;
        tracing::info!("connected to websocket");

        let (mut sink, mut stream) = socket.split();
        let (outgoing, mut queue) = mpsc::unbounded_channel();
        let ctx = Context {
            plugin_id: Arc::from(cfg.id.as_str()),
            outgoing,
            state: Arc::new(Mutex::new(State {
                readiness: cfg.readiness,
                friendly_name: cfg.friendly_name.clone(),
            })),
        };
        let handler = Arc::new(handler);

        // Report the state upon startup.
        ctx.send(ctx.state_response())?;

        let startup = {
            let handler = Arc::clone(&handler);
            let ctx = ctx.clone();
            tokio::spawn(async move {
                if let Err(err) = handler.on_connected(&ctx).await {
                    tracing::error!("on_connected failed: {err}");
                }
            })
        };

        let result = loop {
            tokio::select! {
                Some(frame) = queue.recv() => {
                    if let Err(err) = sink.send(frame).await {
                        break Err(err.into());
                    }
                }
                incoming = stream.next() => match incoming {
                    None => break Ok(()),
                    Some(Err(err)) => break Err(err.into()),
                    Some(Ok(WsMessage::Text(text))) => {
                        tracing::debug!("[RECEIVE] {}", text.as_str());
                        dispatch(&handler, &ctx, text.as_str());
                    }
                    Some(Ok(WsMessage::Ping(_))) => {
                        // tungstenite queues the pong; flush it right away.
                        if let Err(err) = sink.flush().await {
                            break Err(err.into());
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        tracing::info!("websocket closed by remote host");
                        break Ok(());
                    }
                    Some(Ok(_)) => {}
                }
            }
        };

        startup.abort();
        let _ = sink.close().await;
        result
    }
}

fn dispatch<H: Handler>(handler: &Arc<H>, ctx: &Context, text: &str) {
    let message = match Message::from_json(text) {
        Ok(message) => message,
        Err(Error::UnknownMessageType(ty)) => {
            tracing::warn!("ignoring message of unknown type: {ty}");
            return;
        }
        Err(err) => {
            tracing::warn!("error deserializing message: {err}");
            return;
        }
    };

    let handler = Arc::clone(handler);
    let ctx = ctx.clone();
    let id = message.id;
    tokio::spawn(async move {
        let kind = message.payload.type_name();
        let result = match message.payload {
            Payload::PluginStateRequest(b) => handler.on_plugin_state_request(&ctx, id, b).await,
            Payload::ConfigTemplateRequest(b) => {
                handler.on_config_template_request(&ctx, id, b).await
            }
            Payload::ConfigUpdateRequest(b) => handler.on_config_update_request(&ctx, id, b).await,
            Payload::ControlRequest(b) => handler.on_control_request(&ctx, id, b).await,
            Payload::DiscoverRequest(b) => handler.on_discover_request(&ctx, id, b).await,
            Payload::InclusionEvent(b) => handler.on_inclusion_event(&ctx, id, b).await,
            Payload::ExclusionEvent(b) => handler.on_exclusion_event(&ctx, id, b).await,
            Payload::StatusRequest(b) => handler.on_status_request(&ctx, id, b).await,
            Payload::HmipSystemEvent(b) => handler.on_hmip_system_event(&ctx, id, b).await,
            Payload::HmipSystemResponse(b) => handler.on_hmip_system_response(&ctx, id, b).await,
            Payload::SystemInfoResponse(b) => handler.on_system_info_response(&ctx, id, b).await,
            Payload::CreateUserMessageResponse(b) => {
                handler.on_create_user_message_response(&ctx, id, b).await
            }
            Payload::DeleteUserMessageResponse(b) => {
                handler.on_delete_user_message_response(&ctx, id, b).await
            }
            Payload::ListUserMessagesResponse(b) => {
                handler.on_list_user_messages_response(&ctx, id, b).await
            }
            Payload::UserMessageAckEvent(b) => handler.on_user_message_ack_event(&ctx, id, b).await,
            Payload::ErrorResponse(b) => handler.on_error_response(&ctx, id, b).await,
            // Messages only a plugin is supposed to send.
            _ => {
                tracing::debug!("ignoring plugin-to-HCU message type {kind} received from the HCU");
                Ok(())
            }
        };
        if let Err(err) = result {
            tracing::error!("handler for {kind} failed: {err}");
        }
    });
}

/// The HCU presents a self-signed certificate, so certificate validation is disabled — the same
/// trade-off as the other implementations of this library. The connection is only meant to stay
/// inside the host, and the auth token travels in the handshake.
fn insecure_tls_config() -> Result<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = AcceptAnyCert(provider.signature_verification_algorithms);
    Ok(ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth())
}

#[derive(Debug)]
struct AcceptAnyCert(WebPkiSupportedAlgorithms);

impl ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_schemes()
    }
}
