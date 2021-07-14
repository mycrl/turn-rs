mod handshake;

use handshake::Handshake;

pub enum Payload {
    Handshake(Handshake)   
}
