
pub enum Content {
    
}

pub enum Payload {
    
} 

pub struct Dtls {
    content: Content,
    version: u16,
    epoch: u16,
    sequence_number: u64,
    length: u16,
    payload: Vec<Payload>
}