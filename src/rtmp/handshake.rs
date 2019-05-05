use crate::util::rand_strs;
use bytes::BytesMut;


/// # Handshake Info.
pub struct Handshake {
    pub version: u8, // version
    pub types: bool  // is ok
}


impl Handshake {

    /// # Creatd Handshake.
    /// 
    pub fn new () -> Self {
        Handshake { version: 0, types: false }
    }

    /// # Examination Handshake Package.
    /// 
    pub fn then (&self, bytes: &BytesMut) -> (bool, bool) {
        let mut is_type = false;
        let mut is_back = false;
        let mut index = 0;
        let mut offset = 0;

        // examination package length.
        // C0 + C1 || S0 + S1
        if bytes.len() == 1537 {
            // C0, S0
            // lock version number is 3
            if bytes[0] == 3 {
                index = 5;
                offset = 9;
                is_back = true;
            }
        } else {
            index = 4;
            offset = 8;
        }

        // C1, C2
        // S1, S2
        // TODO: check only the default placeholder.
        if index > 0 {
            if bytes[index] == 0 
            && bytes[index + 1] == 0 
            && bytes[index + 2] == 0 
            && bytes[index + 3] == 0 {
                if bytes.len() - offset == 1528 {
                    is_type = true
                }
            }
        }

        // callback type and back.
        (is_type, is_back)
    }

    /// # Create Handshake Package.
    /// S0 + S1 + S2
    /// 
    pub fn created (&self) -> Vec<u8> {
        let mut package = vec![];
        
        // push bytes.
        package.extend_from_slice(&vec![3]); // S0
        package.extend_from_slice(&vec![0; 8]); // S0 head
        package.extend_from_slice(&rand_strs(1528)); // S0 body
        package.extend_from_slice(&vec![0; 8]); // S1 head
        package.extend_from_slice(&rand_strs(1528)); // S1 body
        package
    }
}