// use.
use std::collections::HashMap;
use crate::Tx;


// 客户端
pub struct Client {
    pub uid: String,
    pub tx: Tx
}


// 共享状态
pub struct Shared {
    pub clients: HashMap<String, Vec<Client>>
}


impl Shared {
    pub fn new () -> Self {
        Shared {
            clients: HashMap::new()
        }
    }
}