use rml_amf0::Amf0Value;
use rml_amf0::Amf0Value::{Null, Number, Object, Utf8String};
use rml_rtmp::messages::{PeerBandwidthLimitType::Hard, RtmpMessage};

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
        let mut map = ::std::collections::HashMap::new();
        $( map.insert($key, $val); )*
        map
    }}
}

#[rustfmt::skip]
lazy_static! {
    pub static ref PUBLISH_INFO: Vec<Amf0Value> = vec![Object(hashmap![
        "level".to_string() => Utf8String("status".to_string()),
        "code".to_string() => Utf8String("NetStream.Publish.Start".to_string()),
        "description".to_string() => Utf8String("Start publishing".to_string())
    ])];
}

#[rustfmt::skip]
lazy_static! {
    pub static ref PUBLISH: Vec<RtmpMessage> = vec![RtmpMessage::Amf0Command {
        additional_arguments: PUBLISH_INFO.to_vec(),
        command_name: "onStatus".to_string(), 
        transaction_id: 0.0, 
        command_object: Null,
    }];
}

#[rustfmt::skip]
lazy_static! {
    pub static ref CREATE_STREAM: Vec<RtmpMessage> = vec![RtmpMessage::Amf0Command {
        additional_arguments: vec![Number(1.0)],
        command_name: "_result".to_string(),
        transaction_id: 4.0,
        command_object: Null,
    }];
}

#[rustfmt::skip]
lazy_static! {
    pub static ref CONNECT_SUCCESS: Vec<Amf0Value> = vec![Object(hashmap![
        "level".to_string() => Utf8String("status".to_string()),
        "code".to_string() => Utf8String("NetConnection.Connect.Success".to_string()),
        "description".to_string() => Utf8String("Connection succeeded.".to_string()),
        "objectEncoding".to_string() => Number(0.0)
    ])];
}

#[rustfmt::skip]
lazy_static! {
    pub static ref CONNECT: Vec<RtmpMessage> = vec![
        RtmpMessage::WindowAcknowledgement { size: 5000000 },
        RtmpMessage::SetPeerBandwidth { size: 5000000, limit_type: Hard },
        RtmpMessage::SetChunkSize { size: 4096 },
        RtmpMessage::Amf0Command { 
        additional_arguments: CONNECT_SUCCESS.to_vec(),
        command_name: "_result".to_string(),
        transaction_id: 1.0,
        command_object: Object(hashmap![
        "fmsVer".to_string() => Utf8String("FMS/3,0,1,123".to_string()),
        "capabilities".to_string() => Number(31.0)])}
    ];
}
