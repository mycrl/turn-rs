// use.
use bytes::BytesMut;
use crate::util::rand_numbers;


/// # Handshake Info.
pub struct Handshake {
    pub version: u8, // version
    pub completed: bool  // is ok
}


impl Handshake {

    /// # Creatd Handshake.
    /// 
    pub fn new () -> Self {
        Handshake { version: 0, completed: false }
    }

    /// # Examination Handshake Package.
    /// 
    pub fn then (&self, bytes: &BytesMut) -> (bool, bool) {
        let mut is_type = false;
        let mut is_back = false;
        let mut index = 0;

        // examination package length.
        // C0 + C1 || S0 + S1
        if bytes.len() == 1537 {
            // C0, S0
            // lock version number is 3
            if bytes[0] == 3 {
                index = 5;
                is_back = true;
            }
        } else {
            index = 4;
        }

        // C1, C2
        // S1, S2
        // TODO: check only the default placeholder.
        if index > 0 {
            if bytes[index] == 0 
            && bytes[index + 1] == 0 
            && bytes[index + 2] == 0 
            && bytes[index + 3] == 0 {
                is_type = true
            }
        }

        // callback type and back.
        (is_type, is_back)
    }

    /// # Create Handshake Package.
    /// S0 + S1 + S2
    /// 
    pub fn created (&self) -> BytesMut {
        let mut package = vec![];
        
        // push bytes.
        package.extend_from_slice(&vec![3]); // S0
        package.extend_from_slice(&vec![0; 8]); // S0 head
        package.extend_from_slice(&rand_numbers(1528)); // S0 body
        package.extend_from_slice(&vec![0; 8]); // S1 head
        package.extend_from_slice(&rand_numbers(1528)); // S1 body
        BytesMut::from(package)
    }

    /// # Drop Handshake Package.
    /// 
    pub fn drop (&self, bytes: &BytesMut) -> BytesMut {
        let mut back: Vec<u8> = vec![];
        let mut is_bool = false;
        let mut is_type = false;
        
        // check length.
        if bytes.len() >= 1536 {
            let (left, _) = &bytes.split_at(1536);
            let (types, _) = self.then(&BytesMut::from(*left));
            is_type = types;
        }

        // check is handshake package.
        if is_type == true {
            let (_, right) = &bytes.split_at(1536);
            back = Vec::from(*right);
            is_bool = true;
        }

        // check is split.
        match is_bool {
            true => BytesMut::from(back),
            false => bytes.clone()
        }
    }

    /// # Check if need to handle the handshake.
    /// 
    pub fn metch (&mut self, bytes: &BytesMut) -> (BytesMut, bool) {
        let (is_type, is_back) = self.then(&bytes);
        let mut back = BytesMut::new();

        // need callback handshake.
        if is_type == true && is_back == true {
            back = self.created();
        }

        // not callback handshake.
        // drop handshake package.
        if is_type == true && is_back == false {
            back = self.drop(bytes);
            self.completed = true;
        }

        (back, is_back)
    }
}