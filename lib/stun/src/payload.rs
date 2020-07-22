use std::net::SocketAddr;
use stun_codec::rfc5389::Attribute;
use stun_codec::rfc5389::attributes::{XorMappedAddress, MappedAddress, Software};
use stun_codec::{MessageClass, TransactionId, Method};
use bytecodec::Result;

pub type Message = stun_codec::Message<Attribute>;

fn response(source: SocketAddr, class: MessageClass, method: Method, transaction: TransactionId) -> Result<Message> {
    let mut message = Message::new(class, method, transaction);
    message.add_attribute(Attribute::XorMappedAddress(XorMappedAddress::new(source)));
    message.add_attribute(Attribute::MappedAddress(MappedAddress::new(source)));
    message.add_attribute(Attribute::Software(Software::new("None".to_string())?));
    Ok(message)
}

pub fn process(source: SocketAddr, message: Message) -> Result<Message> {
    let method = Method::new(0x0101)?;
    let class = MessageClass::SuccessResponse;
    let transaction = message.transaction_id();
    Ok(response(source, class, method, transaction)?)
}
