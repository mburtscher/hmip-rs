#![allow(clippy::result_large_err)]

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use hmip_rs::api::*;
use hmip_rs::domain::Device;
use hmip_rs::domain::types::DeviceType;
use hmip_rs::{Context, Handler, Plugin, Result};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

struct Echo;

impl Handler for Echo {
    async fn on_discover_request(
        &self,
        ctx: &Context,
        id: Option<String>,
        _: DiscoverRequestBody,
    ) -> Result<()> {
        let device = Device::new(DeviceType::Switch)
            .friendly_name("Plug")
            .feature(hmip_rs::domain::features::SwitchStateFeature { on: Some(true) });
        ctx.reply(
            id,
            DiscoverResponseBody {
                success: true,
                devices: vec![device],
                error: None,
            },
        )
    }
}

#[tokio::test]
async fn announces_readiness_and_answers_discovery() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        let mut ws =
            tokio_tungstenite::accept_hdr_async(stream, |req: &Request, resp: Response| {
                for name in ["authtoken", "plugin-id", "hmip-system-events"] {
                    headers.push(req.headers()[name].to_str().unwrap().to_owned());
                }
                Ok(resp)
            })
            .await
            .unwrap();

        let Ws::Text(first) = ws.next().await.unwrap().unwrap() else {
            panic!()
        };
        let ready = Message::from_json(first.as_str()).unwrap();
        assert!(matches!(ready.payload, Payload::PluginStateResponse(_)));
        assert_eq!(ready.plugin_id.as_deref(), Some("test.plugin"));

        let request_id = "req-1".to_owned();
        let request = Message::new(DiscoverRequestBody {}).with_id(request_id.clone());
        ws.send(Ws::text(request.to_json().unwrap())).await.unwrap();
        ws.send(Ws::text(r#"{"type":"SOMETHING_NEW","body":{}}"#))
            .await
            .unwrap();

        let Ws::Text(reply) = ws.next().await.unwrap().unwrap() else {
            panic!()
        };
        let reply = Message::from_json(reply.as_str()).unwrap();
        assert_eq!(reply.id, Some(request_id));
        let Payload::DiscoverResponse(body) = reply.payload else {
            panic!()
        };
        assert!(body.success);
        assert_eq!(body.devices[0].friendly_name.as_deref(), Some("Plug"));

        ws.close(None).await.unwrap();
        headers
    });

    let plugin = Plugin::builder("test.plugin")
        .host("127.0.0.1")
        .port(port)
        .token("secret")
        .plaintext()
        .hmip_system_events(true)
        .build();
    tokio::time::timeout(Duration::from_secs(5), plugin.run(Echo))
        .await
        .expect("plugin should stop after close")
        .unwrap();

    assert_eq!(server.await.unwrap(), ["secret", "test.plugin", "true"]);
}

#[tokio::test]
async fn missing_token_is_an_error() {
    let plugin = Plugin::builder("p")
        .token_path("/nonexistent/TOKEN")
        .build();
    let err = plugin.run(()).await.unwrap_err();
    assert!(matches!(err, hmip_rs::Error::MissingToken { .. }));
}

struct Pusher;

impl Handler for Pusher {
    async fn on_connected(&self, ctx: &Context) -> Result<()> {
        ctx.send(StatusEventBody {
            device_id: "d1".into(),
            features: vec![],
        })
    }
}

#[tokio::test]
async fn on_connected_can_push_unsolicited_messages() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let mut types = Vec::new();
        for _ in 0..2 {
            let Ws::Text(text) = ws.next().await.unwrap().unwrap() else {
                panic!()
            };
            let msg = Message::from_json(text.as_str()).unwrap();
            assert!(msg.id.is_some(), "unsolicited messages get a generated id");
            types.push(msg.payload.type_name());
        }
        ws.close(None).await.unwrap();
        types
    });
    let plugin = Plugin::builder("p")
        .host("127.0.0.1")
        .port(port)
        .token("t")
        .plaintext()
        .build();
    tokio::time::timeout(Duration::from_secs(5), plugin.run(Pusher))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        server.await.unwrap(),
        ["PLUGIN_STATE_RESPONSE", "STATUS_EVENT"]
    );
}
