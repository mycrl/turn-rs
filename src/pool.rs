use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::client::Client;
use crate::stream::Stream;


/// Streaming pool pool error definition.
pub enum PoolError {
    NotFund         // Stream not found.
}


/// Streaming pool.
pub struct Pool {
    pub clients: Arc<RwLock<HashMap<String, Vec<Client>>>>, // Client List.
    pub streams: Arc<RwLock<HashMap<String, Stream>>>       // Stream list.
}


impl Pool {

    /// # Create a streaming pool.
    pub fn new () -> Self {
        Pool {
            clients: Arc::new(RwLock::new(HashMap::new())),
            streams: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    /// # Get keywords.
    /// Combine stream name and stream key.
    pub fn get_key (name: &String, stream_key: &String) -> String {
        format!("{}@{}", name, stream_key)
    }

    /// # Whether the stream exists.
    pub fn entry_stream (&self, key: String) -> bool {
        match self.streams.read().get(&key) {
            Some(_) => true,
            None => false
        }
    }

    /// # Add client.
    /// Add the streaming client to the client list.
    pub fn append_client (&self, client: Client) -> Result<(), PoolError> {
        let mut clients = self.clients.write();
        let key = Pool::get_key(&client.name, &client.stream_key);
        match clients.get_mut(&key) {

            // If a group already exists.
            // Add to current group.
            Some(x) => { 
                x.push(client); 
            },

            // If the group does not exist.
            None => {

                // First check if there is already a stream.
                // Create a new group if there is a stream.
                if self.entry_stream(key) {
                    clients.insert(key, vec![ client ]); 
                } else {

                    // This is no stream.
                    // If there is no stream.
                    // return an error indicating that the stream was not found.
                    Err(PoolError::NotFund);
                }
            }
        };

        // emancipation ownership.
        // Apply changes to the list of instances.
        *clients;
        Ok(())
    }

    /// # Push media.
    /// By specified keyword.
    /// Push streaming data to all associated clients.
    pub fn push_matedata (&self, key: String, data: Vec<u8>) {
        let clients = self.clients.read();
        match clients.get(&key) {

            // Client group exists.
            // Notify all clients.
            Some(x) => {
                for client in x {

                }
            },

            // Client group does not exist.
            // Please note that there is no online client for the current channel.
            // drop media data.
            None => {
                drop(data);
            }
        };
    }

    /// # Add stream.
    /// Add a stream to the stream list.
    pub fn append_stream (&self, stream: Stream) {
        let mut streams = self.streams.write();
        let key = Pool::get_key(&stream.name, &stream.stream_key);
        streams.entry(key).or_insert(stream);
        *streams;
    }

    /// # Close Stream.
    /// Stream is closed.
    pub fn drop_stream (&self, key: String) {
        let mut streams = self.streams.write();
        streams.remove(&key);
        *streams;

        // Notify all client streams for the current group to be closed.
        let clients = self.clients.read();
        match clients.get(&key) {
            
            // Find the current group.
            // Notify all clients.
            Some(x) => {
                for client in x {

                }
            },

            // If there is no group.
            // This is ideal, don't have to do anything.
            None => ()
        }
    }
}