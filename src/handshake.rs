use bytes::Bytes;

pub enum Packet {
    CS1,
    CS2
}

pub struct Handshake {
    complete: bool
}

impl Handshake {
    pub fn new () -> Self {
        Self { complete: false }
    }

    fn packet_type (self, data: Bytes) -> Option<Packet> {
        if data.len() == 1537 && data[5] == 0 && data[6] == 0 && data[7] == 0 && data[8] == 0 {
            Some(Packet::CS1)
        } else
        if data.len() == 1536 {
            Some(Packet::CS2)
        } else {
            None
        }
    }

    fn create_packet (self, packet: Packet, data: &[u8]) -> Bytes {
        match packet {
            Packet::CS1 => Bytes::from_static(vec!([
                data[0],
                data[1],
                data[2],
                data[3],
                data[4],
                0, 0, 0, 0
            ]).as_slice())
        }
    }

    pub fn process (data: Bytes) {

    }
}
