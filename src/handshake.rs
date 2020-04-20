use bytes::Bytes;

pub enum Packet {
    CS1,
    CS2,
}

pub struct Handshake {
    pub complete: bool,
}

impl Handshake {
    pub fn new() -> Self {
        Self { complete: false }
    }

    fn packet_type(&mut self, data: &Bytes) -> Option<Packet> {
        if data.len() == 1537 && data[5] == 0 && data[6] == 0 && data[7] == 0 && data[8] == 0 {
            Some(Packet::CS1)
        } else if data.len() == 1536 {
            Some(Packet::CS2)
        } else {
            None
        }
    }

    fn create_packet(&mut self, packet: Packet, data: &Bytes) -> Bytes {
        let mut result = match packet {
            Packet::CS1 => Bytes::from(vec![
                data[0], data[1], data[2], data[3], data[4], 0, 0, 0, 0,
            ]),
            Packet::CS2 => Bytes::from(vec![
                data[0], data[1], data[2], data[3], data[4], 0, 0, 0, 0,
            ]),
        };

        result.extend_from_slice(&[0u8; 1528]);
        result
    }

    pub fn process(&mut self, data: Bytes) -> Option<Bytes> {
        match self.complete {
            false => match self.packet_type(&data) {
                Some(genre) => {
                    if let Packet::CS2 = genre {
                        self.complete = true
                    }

                    let result = self.create_packet(genre, &data);
                    Some(result)
                },
                None => None,
            },
            true => None,
        }
    }
}
