// use.
use httparse::Request;
use std::collections::HashMap;
use sha1::Sha1;


/// # Websocket Handshake.
pub struct Handshake {
    pub completed: bool,
    pub headers: HashMap<String, String>
}


impl Handshake {

    /// # Create handshake instance. 
    pub fn new () -> Self {
        Handshake {
            completed: false,
            headers: HashMap::new()
        }
    }

    /// # Check request header parsing is complete.
    /// first check whether there is data in the header cache.
    /// then check if the request header comes with an upgrade protocol.
    fn parse_is_completed (&self) -> bool {
        match self.headers.len() {
            0 => false,
            _ => match self.headers.get(&String::from("Upgrade")) {
                Some(x) => x == &String::from("websocket"),
                None => false
            }
        }
    }

    /// # Generate the HTTP response that needs to be replied to the client.
    fn generate_res (&self) -> Option<String> {
        match self.generate_key() {
            Some(key) => {
                let mut res_data = String::new();

                for value in vec![
                    "HTTP/1.1 101 Switching Protocols\r\n",
                    "Connection: Upgrade\r\n",
                    "Server: Quasipaa\r\n",
                    "Upgrade: WebSocket\r\n",
                    "Date: {}\r\n",
                    "Access-Control-Allow-Credentials: true\r\n",
                    "Access-Control-Allow-Headers: content-type\r\n",
                    format!("Sec-WebSocket-Accept: {}\r\n\r\n", key).as_str(),
                ] {
                    res_data.push_str(value);
                }

                Some(res_data)
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
            let mut key = x.clone();
            key.push_str("258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
            let mut sha = Sha1::new();
            sha.update(key.as_bytes());
            let digest = sha.digest().to_string();
            Some(base64::encode(digest.as_bytes()))
        } else {
            None
        }
    }

    /// # Parse the HTTP upgrade request.
    fn parse (&mut self, bytes: Vec<u8>) {
        let mut headers = [ httparse::EMPTY_HEADER; 16 ];
        let mut req = Request::new(&mut headers);
        req.parse(&bytes).unwrap();

        // Traversing the request header group.
        for x in headers.iter() {
            if x.name != "" {
                break;
            }

            // Write to the instance header cache.
            let key = String::from(x.name);
            let value = String::from_utf8_lossy(x.value).to_string();
            self.headers.insert(key, value);
        }
    }

    /// # Processing request data.
    pub fn process (&mut self, bytes: Vec<u8>) {
        
    }
}