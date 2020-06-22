use std::net::SocketAddr;
use std::collections::HashMap;
use bytecodec::{Error, ErrorKind};
use stun_codec::{MessageClass, Method, rfc5389::Attribute};
use stun_codec::rfc5389::attributes::{
    XorMappedAddress,
    MappedAddress,
    Software,
    Username,
    Realm,
    Nonce,
    MessageIntegrity
};

pub type Message = stun_codec::Message<Attribute>;
pub struct Payload {
    password: HashMap<String, String>
}

impl Payload {
    fn binding_request(self, message: Message, addr: SocketAddr) -> Result<Message, Error> {
        let id = message.transaction_id();
        let class = MessageClass::SuccessResponse;
        let mut response = Message::new(class, Method::new(0x1010)?, id);
        response.add_attribute(Attribute::XorMappedAddress(XorMappedAddress::new(addr)));
        response.add_attribute(Attribute::MappedAddress(MappedAddress::new(addr)));
        response.add_attribute(Attribute::Software(Software::new("None".to_string())?));
        Ok(response)
    }

    fn allocate_request(&mut self, message: Message, addr: SocketAddr) -> Result<Message, Error> {
        if let  Some(username) = message.get_attribute::<Username>() {
            if let Some(realm) = message.get_attribute::<Realm>() {
                if let Some(nonce) = message.get_attribute::<Nonce>() {
                    if let Some(integrity) =  message.get_attribute::<MessageIntegrity>() {
                        if let Some(password) = self.password.get(username.name()) {
                            if let Ok(_) = integrity.check_long_term_credential(username, realm, &password) {

                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Payload {
    pub fn process(&mut self, message: Message, addr: SocketAddr) -> Result<Message, Error> {
        match message.method().as_u16() {
            0x0001 => self.binding_request(message, addr),
            0x0003 => self.allocate_request(message, addr),
            _ => Err(Error::from(ErrorKind::Other))
        }
    }
}
