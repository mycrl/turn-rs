mod binding;

use bytes::BytesMut;
use stun_codec::rfc5389::Attribute;

pub type Message = stun_codec::Message<Attribute>;

pub struct Payload {
    
}

impl Payload {
    pub fn new() -> Self {
        Self {}
    }

    pub fn decode(&self, message: Message) {
        let method = message.method().as_u16();
        match method {
            0x0001 => {

            },
            _ => {

            }
        };
    }
}
