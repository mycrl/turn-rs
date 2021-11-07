use tokio::time::Instant;
use std::sync::Arc;

/// turn node session.
///
/// * the authentication information.
/// * the port bind table.
/// * the channel alloc table.
/// * the group number.
/// * the time-to-expiry for each relayed transport address.
pub struct Node {
    pub channels: Vec<u16>,
    pub ports: Vec<u16>,
    pub group: u32,
    pub timer: Instant,
    pub lifetime: u64,
    pub password: Arc<[u8; 16]>,
    pub username: String,
}

impl Node {
    /// create node session.
    ///
    /// node session from group number and long key.
    ///
    /// ```no_run
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// // Node::new(0, key.clone());
    /// ```
    pub fn new(
        group: u32, 
        username: String,
        password: [u8; 16]
    ) -> Self {
        Self {
            channels: Vec::with_capacity(5),
            ports: Vec::with_capacity(10),
            timer: Instant::now(),
            password: Arc::new(password),
            lifetime: 600,
            username,
            group,
        }
    }

    /// set the lifetime of the node.
    ///
    /// delay is to die after the specified second.
    ///
    /// ```no_run
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let mut node = Node::new(0, key.clone());
    /// node.set_lifetime(600);
    /// ```
    pub fn set_lifetime(&mut self, delay: u32) {
        self.lifetime = delay as u64;
        self.timer = Instant::now();
    }

    /// whether the node is dead.
    ///
    /// ```no_run
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let mut node = Node::new(0, key.clone());
    /// node.set_lifetime(600);
    /// assert!(!node.is_death());
    /// ```
    pub fn is_death(&self) -> bool {
        self.timer.elapsed().as_secs() >= self.lifetime
    }

    /// get node the password.
    ///
    /// for security reasons, the server MUST NOT store the password
    /// explicitly and MUST store the key value, which is a cryptographic
    /// hash over the username, realm, and password.
    ///
    /// ```no_run
    /// let key = stun::util::long_key("panda", "panda", "raspberry");
    /// let node = Node::new(0, key.clone());
    /// assert_eq!(!node.get_password(), Arc::new(key));
    /// ```
    pub fn get_password(&self) -> Arc<[u8; 16]> {
        self.password.clone()
    }
}
