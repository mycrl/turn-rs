use rml_rtmp::messages::{PeerBandwidthLimitType, RtmpMessage};
use std::collections::HashMap;
use rml_amf0::Amf0Value;

lazy_static! {
    pub static ref MESSAGES: Vec<RtmpMessage> = {
        let mut fms_version = HashMap::new();
        fms_version.insert(String::from("fmsVer"), Amf0Value::Utf8String(String::from("FMS/3,0,1,123")));
        fms_version.insert(String::from("capabilities"), Amf0Value::Number(31.0));

        let mut connect_info = HashMap::new();
        connect_info.insert(String::from("level"), Amf0Value::Utf8String(String::from("status")));
        connect_info.insert(String::from("code"), Amf0Value::Utf8String(String::from("NetConnection.Connect.Success")));
        connect_info.insert(String::from("description"), Amf0Value::Utf8String(String::from("Connection succeeded.")));
        connect_info.insert(String::from("objectEncoding"), Amf0Value::Number(0.0));

        vec![
            RtmpMessage::WindowAcknowledgement { size: 5000000 },
            RtmpMessage::SetPeerBandwidth {
                size: 5000000,
                limit_type: PeerBandwidthLimitType::Hard,
            },
            RtmpMessage::SetChunkSize { size: 2048 },
            RtmpMessage::Amf0Command {
                command_name: String::from("_result"),
                transaction_id: 1.0,
                command_object: Amf0Value::Object(fms_version),
                additional_arguments: vec![ Amf0Value::Object(connect_info) ]
            }
        ]
    };
}