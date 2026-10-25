use hmip_rs::api::*;
use hmip_rs::domain::features::*;
use hmip_rs::domain::types::*;
use hmip_rs::domain::{Device, ErrorInfo};
use serde_json::json;

#[test]
fn discover_request_without_body_parses() {
    let msg = Message::from_json(
        r#"{"id":"6f1c5f56-0c1e-4f56-8c2e-5d6b0b3b7a11","pluginId":"p","type":"DISCOVER_REQUEST"}"#,
    )
    .unwrap();
    assert!(matches!(msg.payload, Payload::DiscoverRequest(_)));
    assert_eq!(msg.plugin_id.as_deref(), Some("p"));
}

#[test]
fn unknown_type_is_reported() {
    let err = Message::from_json(r#"{"type":"NOPE","body":{}}"#).unwrap_err();
    assert!(matches!(err, hmip_rs::Error::UnknownMessageType(t) if t == "NOPE"));
}

#[test]
fn envelope_serializes_like_the_csharp_library() {
    let msg = Message::new(PluginStateResponseBody::new(PluginReadinessStatus::Ready))
        .with_id("00000000-0000-0000-0000-000000000000");
    let value: serde_json::Value = serde_json::from_str(&msg.to_json().unwrap()).unwrap();
    assert_eq!(
        value,
        json!({
            "id": "00000000-0000-0000-0000-000000000000",
            "type": "PLUGIN_STATE_RESPONSE",
            "body": {"pluginReadinessStatus": "READY"}
        })
    );
}

#[test]
fn device_roundtrip_with_features() {
    let device = Device::new(DeviceType::EvCharger)
        .friendly_name("Wallbox")
        .feature(CurrentPowerFeature {
            current_power: Some(-11000.5),
        })
        .feature(EnergyCounterFeature {
            energy_in: Some(1.5),
            energy_out: None,
        });
    let msg = Message::new(StatusResponseBody {
        success: true,
        devices: vec![device],
        error: Some(ErrorInfo::new("X").message("y")),
    });
    let json = msg.to_json().unwrap();
    assert!(json.contains(r#""type":"energyCounter""#));
    assert!(json.contains(r#""in":1.5"#));
    assert!(json.contains(r#""deviceType":"EV_CHARGER""#));
    assert_eq!(Message::from_json(&json).unwrap(), msg);
}

#[test]
fn validation_rejects_out_of_range_values() {
    let ok = ActualTemperatureFeature {
        actual_temperature: Some(60.0),
    };
    assert!(ok.validate().is_ok());
    let bad: Feature = BatteryStateFeature {
        battery_level: Some(1.5),
        battery_capacity: None,
    }
    .into();
    assert!(bad.validate().is_err());
    assert!(
        Device::new(DeviceType::Battery)
            .feature(HumidityFeature {
                humidity: Some(101)
            })
            .validate()
            .is_err()
    );
}

#[test]
fn all_feature_type_names_match_the_api() {
    let names = [
        "actualTemperature",
        "batteryState",
        "climateOperationMode",
        "co2",
        "color",
        "colorTemperature",
        "contactSensorState",
        "coolingTemperatureOffset",
        "currentPower",
        "dimming",
        "energyCounter",
        "heatingTemperatureOffset",
        "hotWaterBoost",
        "humidity",
        "illumination",
        "maintenance",
        "moistureDetected",
        "onTime",
        "particulateMassOne",
        "particulateMassTen",
        "particulateMassTwoPointFive",
        "particulateTypicalSize",
        "presenceDetected",
        "presenceMode",
        "rainCount",
        "raining",
        "setPointTemperature",
        "shutterDirection",
        "shutterLevel",
        "slatsLevel",
        "smokeAlarm",
        "storm",
        "sunshine",
        "sunshineDuration",
        "supplyTemperature",
        "switchState",
        "vehicleRange",
        "waterlevelDetected",
        "windDirection",
        "windSpeed",
    ];
    for name in names {
        let feature: Feature = serde_json::from_value(json!({ "type": name })).unwrap();
        let back = serde_json::to_value(&feature).unwrap();
        assert_eq!(back["type"], name);
    }
}

#[test]
fn config_update_allows_null_values() {
    let msg = Message::from_json(
        r#"{"type":"CONFIG_UPDATE_REQUEST","body":{"languageCode":"de","properties":{"a":"1","b":null}}}"#,
    )
    .unwrap();
    let Payload::ConfigUpdateRequest(body) = msg.payload else {
        panic!()
    };
    assert!(body.properties.unwrap()["b"].is_null());
}

#[test]
fn config_update_accepts_numbers_and_booleans() {
    let msg = Message::from_json(
        r#"{"type":"CONFIG_UPDATE_REQUEST","body":{"properties":{"s":"x","n":0.5,"i":10,"b":true}}}"#,
    )
    .unwrap();
    let Payload::ConfigUpdateRequest(body) = msg.payload else {
        panic!()
    };
    let props = body.properties.unwrap();
    assert_eq!(props["n"], json!(0.5));
    assert_eq!(props["b"], json!(true));
}

#[test]
fn non_uuid_ids_are_accepted() {
    let msg = Message::from_json(
        r#"{"id":"abc","type":"STATUS_REQUEST","body":{"deviceIds":["example"]}}"#,
    )
    .unwrap();
    assert_eq!(msg.id.as_deref(), Some("abc"));
    let Payload::StatusRequest(body) = msg.payload else {
        panic!()
    };
    assert_eq!(body.device_ids, Some(vec!["example".to_owned()]));
    // omitted means "all devices"
    let Payload::StatusRequest(all) = Message::from_json(r#"{"type":"STATUS_REQUEST"}"#)
        .unwrap()
        .payload
    else {
        panic!()
    };
    assert_eq!(all.device_ids, None);
}

#[test]
fn device_types_enforce_required_and_optional_features() {
    let switch = Device::new(DeviceType::Switch);
    assert!(matches!(
        switch.validate(),
        Err(hmip_rs::Error::MissingFeature { .. })
    ));
    let switch = switch.feature(SwitchStateFeature { on: Some(true) });
    assert!(switch.validate().is_ok());
    let bad = switch
        .clone()
        .feature(HumidityFeature { humidity: Some(5) });
    assert!(matches!(
        bad.validate(),
        Err(hmip_rs::Error::FeatureNotSupported { .. })
    ));
    assert!(
        switch
            .feature(MaintenanceFeature {
                unreach: Some(false),
                ..Default::default()
            })
            .validate()
            .is_ok()
    );
}

#[test]
fn every_documented_message_type_roundtrips() {
    let types = [
        "CONFIG_TEMPLATE_REQUEST",
        "CONFIG_TEMPLATE_RESPONSE",
        "CONFIG_UPDATE_REQUEST",
        "CONFIG_UPDATE_RESPONSE",
        "CONTROL_REQUEST",
        "CONTROL_RESPONSE",
        "CREATE_USER_MESSAGE_REQUEST",
        "CREATE_USER_MESSAGE_RESPONSE",
        "DELETE_USER_MESSAGE_REQUEST",
        "DELETE_USER_MESSAGE_RESPONSE",
        "DISCOVER_REQUEST",
        "DISCOVER_RESPONSE",
        "ERROR_RESPONSE",
        "EXCLUSION_EVENT",
        "HMIP_SYSTEM_EVENT",
        "HMIP_SYSTEM_REQUEST",
        "HMIP_SYSTEM_RESPONSE",
        "INCLUSION_EVENT",
        "LIST_USER_MESSAGES_REQUEST",
        "LIST_USER_MESSAGES_RESPONSE",
        "PLUGIN_STATE_REQUEST",
        "PLUGIN_STATE_RESPONSE",
        "STATUS_EVENT",
        "STATUS_REQUEST",
        "STATUS_RESPONSE",
        "SYSTEM_INFO_REQUEST",
        "SYSTEM_INFO_RESPONSE",
        "USER_MESSAGE_ACK_EVENT",
    ];
    // Minimal bodies satisfying the fields the documentation marks as required.
    let bodies = json!({
        "CONFIG_UPDATE_RESPONSE": {"status": "APPLIED"},
        "CREATE_USER_MESSAGE_REQUEST": {"behaviorType": "DISMISSIBLE", "message": {"en": "m"}, "messageCategory": "INFO", "timestamp": 1, "title": {"en": "t"}, "userMessageId": "u"},
        "ERROR_RESPONSE": {"error": {"code": "ERROR"}, "originalMessage": "Hello"},
        "HMIP_SYSTEM_REQUEST": {"path": "/hmip/home/getSystemState"},
        "PLUGIN_STATE_RESPONSE": {"pluginReadinessStatus": "READY"},
        "USER_MESSAGE_ACK_EVENT": {"ackType": "OK", "userMessageId": "u"},
    });
    for ty in types {
        let body = bodies.get(ty).cloned().unwrap_or_else(|| json!({}));
        let msg = Message::from_json(
            &json!({"id": "1", "pluginId": "p", "type": ty, "body": body}).to_string(),
        )
        .unwrap_or_else(|e| panic!("{ty}: {e}"));
        assert_eq!(msg.payload.type_name(), ty);
        assert_eq!(Message::from_json(&msg.to_json().unwrap()).unwrap(), msg);
    }
}

#[test]
fn documented_examples_parse() {
    let ctl = Message::from_json(r#"{"pluginId":"p","id":"1","type":"CONTROL_REQUEST","body":{"features":[{"dimLevel":0.20,"type":"dimming"},{"on":true,"type":"switchState"}],"deviceId":"c0acf4e7"}}"#).unwrap();
    let Payload::ControlRequest(body) = ctl.payload else {
        panic!()
    };
    assert_eq!(body.features.len(), 2);

    let resp = Message::from_json(r#"{"type":"HMIP_SYSTEM_RESPONSE","body":{"code":400,"body":{"errorCode":"FEATURE_NOT_SUPPORTED"}}}"#).unwrap();
    let Payload::HmipSystemResponse(body) = resp.payload else {
        panic!()
    };
    assert_eq!(body.code, 400);

    let info = Message::from_json(r#"{"type":"SYSTEM_INFO_RESPONSE","body":{"isOnlineMode":true,"success":true,"ipAddress":"example"}}"#).unwrap();
    assert!(
        matches!(info.payload, Payload::SystemInfoResponse(b) if b.is_online_mode && b.ip_address == "example")
    );
}
