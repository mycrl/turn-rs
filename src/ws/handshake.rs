// use.
use bytes::BytesMut;
use std::sync::mpsc::Sender;
use httparse::Request;
use std::collections::HashMap;
use std::time::SystemTime;
use httpdate::fmt_http_date;
use ring::digest;


/// # Websocket Handshake.
pub struct Handshake {
    pub completed: bool,
    pub headers: HashMap<String, String>,
    pub sender: Sender<BytesMut>
}


impl Handshake {

    /// # Create handshake instance. 
    pub fn new (sender: Sender<BytesMut>) -> Self {
        Handshake {
            completed: false,
            headers: HashMap::new(),
            sender: sender
        }
    }

    /// # Generate the HTTP response that needs to be replied to the client.
    fn generate_res (&self) -> Option<String> {
        match self.generate_key() {
            Some(key) => {
                let mut res = String::new();
                res.push_str("HTTP/1.1 101 Switching Protocols\r\n");
                res.push_str("Connection: Upgrade\r\n");
                res.push_str("Server: Quasipaa\r\n");
                res.push_str("Upgrade: WebSocket\r\n");
                res.push_str(format!("Date: {}\r\n", fmt_http_date(SystemTime::now())).as_str());
                res.push_str(format!("Sec-WebSocket-Accept: {}\r\n\r\n", key).as_str());
                Some(res)
            },
            None => None
        }
    }

    /// # Generate the key that needs to be restored to the client.
    /// After the server receives the request, 
    /// it is mainly used to generate a pair of sec-websocket-accept keys for the sec-websocket-key, 
    /// and the simple one is Sha1(sec-websocket-key + 258eafa5-e914-47da-95ca-c5ab0dc85b11).
    fn generate_key (&self) -> Option<String> {
        if let Some(x) = self.headers.get(&String::from("Sec-WebSocket-Key")) {
            let mut key = x.trim().clone().to_string();
            key.push_str("258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
            let dig = digest::digest(&digest::SHA1, key.as_bytes());
            Some(base64::encode(dig.as_ref()))
        } else {
            None
        }
    }

    /// # Parse the HTTP upgrade request.
    fn parse (&mut self, bytes: Vec<u8>) -> bool {
        let mut headers = [ httparse::EMPTY_HEADER; 16 ];
        let mut req = Request::new(&mut headers);
        req.parse(&bytes).unwrap();

        // Traversing the request header group.
        for x in headers.iter() {
            if x.name == "" {
                break;
            }

            // Write to the instance header cache.
            let key = String::from(x.name);
            let value = String::from_utf8_lossy(x.value).to_string();
            self.headers.insert(key, value);
        }

        // Check request header parsing is complete.
        // first check whether there is data in the header cache.
        // then check if the request header comes with an upgrade protocol.
        match self.headers.len() {
            0 => false,
            _ => match self.headers.get(&String::from("Upgrade")) {
                Some(x) => x == &String::from("websocket"),
                None => false
            }
        }
    }

    /// # Processing request data.
    pub fn process (&mut self, bytes: Vec<u8>) {
        if let true = self.parse(bytes) {
            if let Some(x) = self.generate_res() {
                self.sender.send(BytesMut::from(x.as_bytes())).unwrap();
                self.completed = true;
            }
        }
    }
}