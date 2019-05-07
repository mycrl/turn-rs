// use.
use std::collections::HashMap;


/// # Action Message Format.
pub struct AMF {
    pub type_id: u8,
    pub body: Vec<u8>
}


impl AMF {

    /// # Create amf instance.
    /// 
    pub fn create (type_id: &u8, body: &Vec<u8>) -> Self {
        AMF {
            type_id: type_id.clone(),
            body: body.clone()
        }
    }

    /// # Match types.
    ///
    pub fn match_type (&self) {
        println!("AMF type id {:?}", self.type_id);
        println!("AMF body {:?}", self.body);
    }
}