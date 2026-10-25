# hmip-rs

Build plugins for the **HomematicIP Home Control Unit (HCU)** in Rust.

`hmip-rs` implements the websocket protocol of the HomematicIP *Connect API*. It gives you

- **typed messages** for the complete protocol: every message type, device, feature and enum of the [Connect API 1.0.1 documentation](https://github.com/homematicip/connect-api), with serde (de)serialization that matches the wire format;
- a **base plugin implementation** that handles the connection, authentication, state reporting and message dispatch, so a plugin only implements what it actually cares about;
- **validation** of the value ranges and device archetype rules the API documents, before anything is sent to the HCU.

Use it to bring third-party devices into your Homematic IP system (inverters, wallboxes, heat pumps, sensors, …), to offer plugin configuration in HCUweb, to show messages in the Homematic IP app or to control and monitor the Homematic IP system from your own code.

> **Disclaimer:** This project is independently developed and is not associated with, endorsed by, or sponsored by Homematic / eQ-3 AG. For official information, visit their GitHub repositories at <https://github.com/homematicip>.

## Contents

- [Getting started](#getting-started)
- [How the Connect API works](#how-the-connect-api-works)
- [The plugin and its handler](#the-plugin-and-its-handler)
- [Including devices](#including-devices)
- [Configuration in HCUweb](#configuration-in-hcuweb)
- [Plugin state](#plugin-state)
- [Messages in the app](#messages-in-the-app)
- [Controlling and monitoring the Homematic IP system](#controlling-and-monitoring-the-homematic-ip-system)
- [Using only the message types](#using-only-the-message-types)
- [Connection settings and running remotely](#connection-settings-and-running-remotely)
- [Error handling and logging](#error-handling-and-logging)
- [Shipping a plugin](#shipping-a-plugin)
- [Device archetypes and features](#device-archetypes-and-features)

## Getting started

```bash
cargo add hmip-rs
cargo add tokio --features rt-multi-thread,macros
```

The crate is async and runs on [Tokio](https://tokio.rs). It uses rustls (no OpenSSL) and needs Rust 1.85 or newer. The smallest possible plugin connects, reports itself as ready and ignores everything else:

```rust,no_run
use hmip_rs::Plugin;

#[tokio::main]
async fn main() -> hmip_rs::Result<()> {
    // `()` is a handler that only uses the defaults.
    Plugin::new("com.example.hello").run(()).await
}
```

Installed plugins run in a container on the HCU. They find the HCU under `host.containers.internal:9001` and their authorization token in `/TOKEN` – exactly the defaults of `Plugin::new`. To develop on your own machine, see [Connection settings and running remotely](#connection-settings-and-running-remotely).

## How the Connect API works

A plugin opens a **websocket** to the HCU and exchanges JSON messages with it. Every message is an envelope

```json
{ "id": "…", "pluginId": "com.example.hello", "type": "DISCOVER_REQUEST", "body": { } }
```

- `id` correlates a flow: **a response carries the id of its request.** Unsolicited messages get a fresh id (the crate generates a UUID for you).
- `pluginId` identifies the plugin.
- `type` selects the schema of `body`.

The HCU drives most of the conversation (it *requests* things), the plugin answers and may push updates on its own:

| Message type | Direction | Typical use |
|---|---|---|
| `PLUGIN_STATE_REQUEST` / `_RESPONSE` | HCU → plugin → HCU | report readiness (`READY`, `CONFIG_REQUIRED`, `ERROR`) |
| `CONFIG_TEMPLATE_REQUEST` / `_RESPONSE` | HCU → plugin → HCU | describe the configuration form for HCUweb |
| `CONFIG_UPDATE_REQUEST` / `_RESPONSE` | HCU → plugin → HCU | apply a configuration the user saved |
| `DISCOVER_REQUEST` / `_RESPONSE` | HCU → plugin → HCU | list devices available for inclusion |
| `INCLUSION_EVENT`, `EXCLUSION_EVENT` | HCU → plugin | devices were included / removed |
| `STATUS_REQUEST` / `_RESPONSE`, `STATUS_EVENT` | HCU → plugin → HCU, plugin → HCU | report device state, full or partial |
| `CONTROL_REQUEST` / `_RESPONSE` | HCU → plugin → HCU | set target values on a plugin device |
| `HMIP_SYSTEM_REQUEST` / `_RESPONSE`, `HMIP_SYSTEM_EVENT` | plugin → HCU → plugin, HCU → plugin | query and control the Homematic IP system |
| `CREATE_`/`DELETE_`/`LIST_USER_MESSAGE(S)_*`, `USER_MESSAGE_ACK_EVENT` | both | messages in the smartphone app |
| `SYSTEM_INFO_REQUEST` / `_RESPONSE` | plugin → HCU → plugin | IP address and online mode of the HCU |
| `ERROR_RESPONSE` | HCU → plugin | a message of the plugin could not be processed |

The full protocol is described in the official [Connect API documentation](https://github.com/homematicip/connect-api).

## The plugin and its handler

[`Plugin`](https://docs.rs/hmip-rs/latest/hmip_rs/struct.Plugin.html) owns the connection. [`Plugin::run`](https://docs.rs/hmip-rs/latest/hmip_rs/struct.Plugin.html#method.run) connects, immediately sends a `PLUGIN_STATE_RESPONSE`, and then dispatches every incoming message to your implementation of the [`Handler`](https://docs.rs/hmip-rs/latest/hmip_rs/trait.Handler.html) trait until the connection closes.

`Handler` has one method per message the HCU can send (`on_discover_request`, `on_status_request`, `on_control_request`, …). **Every method has a default**, so you override only what your plugin supports. The defaults follow the API's recommendations:

| Message | Default behaviour |
|---|---|
| `PLUGIN_STATE_REQUEST` | answered with the current readiness (see [Plugin state](#plugin-state)) |
| `CONFIG_TEMPLATE_REQUEST` | answered with an empty template |
| everything else | logged at debug level and ignored |

Things worth knowing:

- Handlers are **async methods on `&self`** (`async fn` in traits, no macro needed). Your handler type must be `Send + Sync + 'static`; keep mutable state behind `Arc`, `Mutex`, `RwLock` or channels.
- **Every incoming message is handled in its own task.** A slow handler (a request to a cloud API, say) never blocks other messages or the connection, but handlers can run concurrently.
- A handler returns `hmip_rs::Result<()>`. Returned errors are logged; the connection stays up.
- You answer through the [`Context`](https://docs.rs/hmip-rs/latest/hmip_rs/struct.Context.html) passed to every method. It is cheap to clone and can be sent to other tasks.

`Context` offers:

| Method | Purpose |
|---|---|
| `reply(request_id, body)` | send `body` as the answer to a request – reuses the request's id |
| `send(body_or_message)` | send an unsolicited message; fills in `pluginId` and a random `id` |
| `set_readiness(status)` | record and report a new [plugin state](#plugin-state) |
| `state_response()` | the current `PLUGIN_STATE_RESPONSE` body |
| `plugin_id()` | the id the plugin connected with |

`send` and `reply` accept any body struct (`DiscoverResponseBody`, `StatusEventBody`, …) as well as a complete [`Message`](https://docs.rs/hmip-rs/latest/hmip_rs/api/struct.Message.html). They only *queue* the message, so they are synchronous and never block; transmission happens on the connection task. Device features in discovery/status messages are validated before queueing, so an invalid value fails here instead of at the HCU.

Besides the request handlers there is one lifecycle hook, `on_connected`. It runs once after the connection is up and receives a `Context` – the place to start periodic work (see below). It is cancelled when the connection ends.

## Including devices

To expose third-party devices, answer the discovery and status messages. A device is a [`Device`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/struct.Device.html) of one archetype ([`DeviceType`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/types/enum.DeviceType.html)) with a set of **features** that carry its values.

```rust,no_run
use std::time::Duration;

use hmip_rs::api::*;
use hmip_rs::domain::features::*;
use hmip_rs::domain::types::DeviceType;
use hmip_rs::domain::Device;
use hmip_rs::{Context, Handler, Plugin, Result};

struct Garden;

impl Garden {
    /// Device ids must stay stable across restarts.
    fn devices(&self, temperature: f64) -> Vec<Device> {
        vec![
            Device::with_id("garden-thermometer", DeviceType::ClimateSensor)
                .friendly_name("Garden")
                .model_type("ACME TempSense 3000")
                .feature(ActualTemperatureFeature { actual_temperature: Some(temperature) })
                .feature(HumidityFeature { humidity: Some(61) }),
        ]
    }
}

impl Handler for Garden {
    // HCU → plugin: which devices can be included?
    async fn on_discover_request(&self, ctx: &Context, id: Option<String>, _: DiscoverRequestBody) -> Result<()> {
        ctx.reply(id, DiscoverResponseBody { success: true, devices: self.devices(21.5), error: None })
    }

    // HCU → plugin: these devices are now included – report their state.
    async fn on_inclusion_event(&self, ctx: &Context, id: Option<String>, body: InclusionEventBody) -> Result<()> {
        let devices = self.devices(21.5).into_iter().filter(|d| body.device_ids.contains(&d.device_id)).collect();
        ctx.reply(id, StatusResponseBody { success: true, devices, error: None })
    }

    // HCU → plugin: report the state of some (or all, if `device_ids` is `None`) devices.
    async fn on_status_request(&self, ctx: &Context, id: Option<String>, body: StatusRequestBody) -> Result<()> {
        let devices = self
            .devices(21.5)
            .into_iter()
            .filter(|d| body.device_ids.as_ref().is_none_or(|ids| ids.contains(&d.device_id)))
            .collect();
        ctx.reply(id, StatusResponseBody { success: true, devices, error: None })
    }

    // Push changes proactively: a partial update of a single device.
    async fn on_connected(&self, ctx: &Context) -> Result<()> {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            // Returns an error once the connection is gone, which ends this loop.
            ctx.send(StatusEventBody {
                device_id: "garden-thermometer".into(),
                features: vec![ActualTemperatureFeature { actual_temperature: Some(21.7) }.into()],
            })?;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Plugin::new("com.example.garden").run(Garden).await
}
```

The flow the API intends:

1. The plugin starts and reports `READY` (done by `Plugin::run`).
2. `DISCOVER_REQUEST` → you answer with the available devices.
3. The user includes devices in HCUweb; the HCU sends `INCLUSION_EVENT` → you answer with a `STATUS_RESPONSE` for all included devices.
4. From then on you answer `STATUS_REQUEST`s and **push** `STATUS_RESPONSE`s or `STATUS_EVENT`s when values change.
5. On `EXCLUSION_EVENT` you stop reporting the removed devices.
6. `CONTROL_REQUEST` asks you to set target values (for example `switchState` or `dimming`); you apply them to the real device and answer with a `CONTROL_RESPONSE`.

### Features and validation

A feature is a small struct (`SwitchStateFeature`, `ActualTemperatureFeature`, …) whose fields are all `Option`s – a feature only reports the values it knows about. The [`Feature`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/features/enum.Feature.html) enum is the tagged union that appears on the wire (`{"type": "actualTemperature", …}`); every feature struct converts into it with `.into()`, and `Device::feature` accepts either.

The API documents value ranges (a battery level is 0–1, humidity 0–100, …) and which features each device archetype requires or allows. Both are checked:

- `validate()` on a feature struct or on `Feature` checks ranges,
- `Device::validate()` additionally checks that all **required** features of the archetype are present and that no feature outside the required/optional set is used,
- `Context::send` runs this validation for `DISCOVER_RESPONSE`, `STATUS_RESPONSE` and `STATUS_EVENT`.

Parsing is deliberately lenient: an out-of-range value *received* from the HCU never makes message handling fail.

## Configuration in HCUweb

Plugins can offer a configuration form. Answer `CONFIG_TEMPLATE_REQUEST` with a template (properties, optionally grouped, with types and constraints) and apply the values the user saves in `CONFIG_UPDATE_REQUEST`:

```rust,no_run
use std::collections::HashMap;
use std::sync::Mutex;

use hmip_rs::api::*;
use hmip_rs::domain::types::{ConfigUpdateResponseStatus, PluginReadinessStatus, PropertyType};
use hmip_rs::domain::{GroupTemplate, PropertyTemplate};
use hmip_rs::{Context, Handler, Plugin, Result};

#[derive(Default)]
struct Bridge {
    address: Mutex<Option<String>>,
}

impl Handler for Bridge {
    async fn on_config_template_request(&self, ctx: &Context, id: Option<String>, _: ConfigTemplateRequestBody) -> Result<()> {
        let mut address = PropertyTemplate::new(PropertyType::String, "Bridge address");
        address.description = Some("IP address of the bridge in your network.".into());
        address.group_id = Some("network".into());
        address.current_value = self.address.lock().unwrap().clone();
        address.required = Some(true);

        ctx.reply(
            id,
            ConfigTemplateResponseBody {
                groups: Some(HashMap::from([(
                    "network".to_owned(),
                    GroupTemplate { friendly_name: "Network".into(), ..Default::default() },
                )])),
                properties: HashMap::from([("address".to_owned(), address)]),
            },
        )
    }

    async fn on_config_update_request(&self, ctx: &Context, id: Option<String>, body: ConfigUpdateRequestBody) -> Result<()> {
        let address = body.properties.as_ref().and_then(|p| p.get("address")).and_then(|v| v.as_str());
        let response = match address {
            Some(address) if !address.is_empty() => {
                *self.address.lock().unwrap() = Some(address.to_owned());
                // Persist it (e.g. below /data) and report the plugin as ready.
                ctx.set_readiness(PluginReadinessStatus::Ready)?;
                ConfigUpdateResponseBody { status: ConfigUpdateResponseStatus::Applied, message: None }
            }
            _ => ConfigUpdateResponseBody {
                status: ConfigUpdateResponseStatus::Failed,
                message: Some("Please enter the address of the bridge.".into()),
            },
        };
        ctx.reply(id, response)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Plugin::builder("com.example.bridge")
        .initial_readiness(PluginReadinessStatus::ConfigRequired)
        .build()
        .run(Bridge::default())
        .await
}
```

Notes:

- Property **values arrive as JSON values** (`serde_json::Value`): strings, numbers or booleans depending on the property's [`PropertyType`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/types/enum.PropertyType.html) (`STRING`, `INTEGER`, `NUMBER`, `BOOLEAN`, `ENUM`, `PASSWORD`, `TYPEAHEAD`, `READONLY`, `QRCODE`, `WEBLINK`). A cleared optional property is `null`.
- A `CONFIG_UPDATE_RESPONSE` may be sent several times. Send `PENDING` while you wait for something (for example a button press on a bridge – put instructions into `message`) and `APPLIED` or `FAILED` when you know the result. Messages are shown in HCUweb, in the language of `language_code`.
- A plugin without configuration needs nothing: the default template answer is an empty form.
- Installed plugins can keep their settings in `/data`, which the HCU preserves across updates and restarts.

## Plugin state

The HCU shows each plugin as `READY`, `CONFIG_REQUIRED` or `ERROR` ([`PluginReadinessStatus`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/types/enum.PluginReadinessStatus.html)), together with an optional localized name.

- `PluginBuilder::initial_readiness` sets the state reported on connect (default `READY`); `PluginBuilder::friendly_name("de", "…")` sets the plugin's name (the API requires the `de` entry once any name is given).
- `Context::set_readiness` records a new state *and* reports it. Call it whenever the state changes – for example after a successful configuration update, or when the third-party system becomes unreachable.
- The default `PLUGIN_STATE_REQUEST` handler answers with the recorded state, so you normally never handle that message yourself.

## Messages in the app

Plugins can show messages on the home screen of the Homematic IP smartphone app:

```rust,no_run
use std::collections::HashMap;

use hmip_rs::api::*;
use hmip_rs::domain::types::{BehaviorType, MessageCategory};
use hmip_rs::{Context, Handler, Plugin, Result};

struct Notifier;

impl Handler for Notifier {
    async fn on_connected(&self, ctx: &Context) -> Result<()> {
        ctx.send(CreateUserMessageRequestBody {
            user_message_id: "bridge-offline".into(), // re-sending the same id replaces the message
            message_category: MessageCategory::Error,
            behavior_type: BehaviorType::AcknowledgeableByOk,
            title: HashMap::from([("en".into(), "Connection error".into()), ("de".into(), "Verbindungsfehler".into())]),
            message: HashMap::from([
                ("en".into(), "The bridge cannot be reached.".into()),
                ("de".into(), "Die Bridge ist nicht erreichbar.".into()),
            ]),
            timestamp: 1_700_000_000_000,
        })
    }

    async fn on_user_message_ack_event(&self, ctx: &Context, _id: Option<String>, body: UserMessageAckEventBody) -> Result<()> {
        // The user pressed OK (or YES / NO); remove the message if it is still around.
        ctx.send(DeleteUserMessageRequestBody { user_message_id: body.user_message_id })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Plugin::new("com.example.notifier").run(Notifier).await
}
```

The [`BehaviorType`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/types/enum.BehaviorType.html) decides how the message can be dismissed: `Dismissible` and `NotDismissible` produce no acknowledgement; `AcknowledgeableByOk` and `AcknowledgeableByYesNo` produce a `USER_MESSAGE_ACK_EVENT`. The responses (`on_create_user_message_response`, …) are handler methods as well; `ListUserMessagesRequestBody` retrieves everything the plugin created.

## Controlling and monitoring the Homematic IP system

`HMIP_SYSTEM_REQUEST` is a tunnel into the Homematic IP system: it takes a `path` such as `/hmip/device/control/setSwitchState` and a JSON `body` structured according to that path's schema. The response arrives in `on_hmip_system_response` with the id of your request.

```rust,no_run
use hmip_rs::api::*;
use hmip_rs::{Context, Handler, Plugin, Result};
use serde_json::json;

struct Switcher;

impl Handler for Switcher {
    async fn on_connected(&self, ctx: &Context) -> Result<()> {
        // Fetch the system state once; keep it up to date from HMIP_SYSTEM_EVENTs.
        ctx.send(HmipSystemRequestBody::new("/hmip/home/getSystemState", json!({})))?;
        // Turn a switching actuator on.
        ctx.send(HmipSystemRequestBody::new(
            "/hmip/device/control/setSwitchState",
            json!({ "on": true, "channelIndex": 1, "deviceId": "3014F711A000000000001234" }),
        ))
    }

    async fn on_hmip_system_response(&self, _ctx: &Context, id: Option<String>, body: HmipSystemResponseBody) -> Result<()> {
        println!("request {id:?} finished with code {}", body.code);
        Ok(())
    }

    async fn on_hmip_system_event(&self, _ctx: &Context, _id: Option<String>, body: HmipSystemEventBody) -> Result<()> {
        println!("system changed: {}", body.event_transaction);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Plugin::builder("com.example.switcher")
        .hmip_system_events(true) // subscribe to HMIP_SYSTEM_EVENTs
        .build()
        .run(Switcher)
        .await
}
```

Request bodies and the transaction object of system events are exposed as untyped `serde_json::Value`: the set of system paths is large and defined by the HCU, so the crate does not duplicate it. See the *Homematic IP system requests* and *events* chapters of the Connect API documentation for the available paths and schemas. The same goes for `SYSTEM_INFO_REQUEST`, which returns the HCU's IP address and whether it is in online mode.

## Using only the message types

The [`api`](https://docs.rs/hmip-rs/latest/hmip_rs/api/index.html) and [`domain`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/index.html) modules have no dependency on the connection logic. You can use them to write your own transport, a test double of the HCU, or a protocol inspector:

```rust
use hmip_rs::api::{Message, Payload};

let text = r#"{
    "id": "38967997-e1b3-463f-8dc4-f889bb5d10a2",
    "pluginId": "com.example.hello",
    "type": "CONFIG_TEMPLATE_REQUEST",
    "body": { "languageCode": "de" }
}"#;

let message = Message::from_json(text).unwrap();
match &message.payload {
    Payload::ConfigTemplateRequest(body) => assert_eq!(body.language_code.as_deref(), Some("de")),
    other => panic!("unexpected {}", other.type_name()),
}

// And back: a message without ids gets them from `Context::send`; here we set them ourselves.
let json = message.to_json().unwrap();
assert!(json.contains(r#""type":"CONFIG_TEMPLATE_REQUEST""#));
```

`Message::from_json` treats a missing `body` as an empty one and reports unknown `type`s as `Error::UnknownMessageType` – distinct from malformed JSON – so you can ignore message types of newer API versions. [`Payload`](https://docs.rs/hmip-rs/latest/hmip_rs/api/enum.Payload.html) and the enums are `#[non_exhaustive]`; keep a wildcard arm in your matches.

## Connection settings and running remotely

`Plugin::new(id)` uses the defaults for an installed plugin. [`Plugin::builder`](https://docs.rs/hmip-rs/latest/hmip_rs/struct.Plugin.html#method.builder) changes them:

| Setting | Default | Notes |
|---|---|---|
| `host` | `host.containers.internal` | remote plugins use `hcu1-XXXX.local` (last four digits of the HCU's SGTIN) |
| `port` | `9001` | |
| `token` | contents of the token file | |
| `token_path` | `/TOKEN` | trimmed; used when no token is set |
| `initial_readiness` | `READY` | |
| `friendly_name(lang, name)` | – | plugin name per language |
| `hmip_system_events(bool)` | `false` | subscribe to system events |
| `plaintext()` | TLS on | `ws://` instead of `wss://`; for tests against a local server only |

To **run and debug on your own computer** instead of inside the HCU (the code stays the same):

1. In HCUweb, activate the developer mode and expose the Connect API websocket in the local network.
2. Choose a plugin id and request an authorization token for it as described in the Connect API documentation, *Getting Started*.
3. Run with the builder:

```rust,no_run
use hmip_rs::Plugin;

#[tokio::main]
async fn main() -> hmip_rs::Result<()> {
    Plugin::builder("com.example.hello")
        .host("hcu1-1234.local")
        .token(std::env::var("HCU_TOKEN").expect("set HCU_TOKEN"))
        .build()
        .run(())
        .await
}
```

The HCU uses a self-signed certificate. As the Connect API documentation allows, the crate **does not validate the server certificate** (the token is the credential). Do not point a plugin at hosts other than your own HCU.

`Plugin::run` returns `Ok(())` when the HCU closes the connection cleanly and an error otherwise. It does not reconnect; installed plugins are supervised by the HCU, which restarts the container. To stop a plugin, drop the future (for example with `tokio::select!` against a signal).

## Error handling and logging

Everything fallible returns [`hmip_rs::Error`](https://docs.rs/hmip-rs/latest/hmip_rs/enum.Error.html) (`#[non_exhaustive]`): missing token, validation failures (`OutOfRange`, `FeatureNotSupported`, `MissingFeature`), JSON, websocket, TLS and I/O errors, and `NotConnected` once the connection is gone – which is what ends loops that call `ctx.send(…)?`.

The crate logs through [`tracing`](https://docs.rs/tracing). Install any subscriber (for example `tracing-subscriber`) to see connection events, warnings about messages that could not be parsed, handler errors, and – at `debug` level – every sent and received message.

## Shipping a plugin

Installed plugins are **ARM64 container images** with a `de.eq3.hmip.plugin.metadata` label, uploaded to HCUweb as a `.tar.gz`. The [repository this crate lives in](https://github.com/mburtscher/HomematicIP.Net) contains a ready-made pipeline: a script that cross-compiles a plugin to one static, size-optimized binary, puts it into a `scratch` image with the correct label and tags, and exports the archive. See the Fronius plugin there for a complete example.

## Device archetypes and features

Required and optional features per [`DeviceType`](https://docs.rs/hmip-rs/latest/hmip_rs/domain/types/enum.DeviceType.html) (also available as `DeviceType::required_features()` / `optional_features()`; checked by `Device::validate`):

| Archetype | Required | Optional |
|---|---|---|
| `BATTERY` | `batteryState` | `maintenance`, `currentPower`, `energyCounter` |
| `CLIMATE_SENSOR` | – | `actualTemperature`, `humidity`, `illumination`, `co2`, `windSpeed`, `windDirection`, `storm`, `sunshine`, `sunshineDuration`, `raining`, `rainCount`, `maintenance` |
| `CONTACT_SENSOR` | `contactSensorState` | `maintenance` |
| `ENERGY_METER`, `EV_CHARGER`, `GRID_CONNECTION_POINT`, `HVAC`, `INVERTER` | `currentPower` | `energyCounter`, `maintenance` |
| `HEAT_PUMP` | `climateOperationMode` | `coolingTemperatureOffset`, `heatingTemperatureOffset`, `presenceMode`, `hotWaterBoost`, `supplyTemperature`, `maintenance` |
| `LIGHT` | `switchState` | `dimming`, `color`, `colorTemperature`, `onTime`, `maintenance` |
| `OCCUPANCY_SENSOR` | `presenceDetected` | `maintenance` |
| `PARTICULATE_MATTER_SENSOR` | – | `particulateMassOne`, `particulateMassTwoPointFive`, `particulateMassTen`, `particulateTypicalSize`, `actualTemperature`, `humidity`, `maintenance` |
| `SMOKE_ALARM` | `smokeAlarm` | `maintenance` |
| `SWITCH` | `switchState` | `onTime`, `maintenance` |
| `SWITCH_INPUT` | – | `maintenance` |
| `THERMOSTAT` | `setPointTemperature` | `actualTemperature`, `humidity`, `co2`, `maintenance` |
| `VEHICLE` | `batteryState` | `vehicleRange`, `maintenance` |
| `WATER_SENSOR` | `waterlevelDetected` | `moistureDetected`, `maintenance` |
| `WINDOW_COVERING` | `shutterLevel` | `slatsLevel`, `shutterDirection`, `maintenance` |

## License

MIT. See the [LICENSE](https://github.com/mburtscher/HomematicIP.Net/blob/main/LICENSE) file.
