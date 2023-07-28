use crate::router::Router;

use std::sync::{Arc, Mutex};

use bytes::BytesMut;
use faster_stun::attribute::*;
use faster_stun::*;
use turn_proxy::ProxyObserver;
use turn_rs::{Service, StunClass};

pub struct ProxyExt {
    service: Service,
    router: Arc<Router>,
    decoder: Mutex<Decoder>,
    buf: Mutex<BytesMut>,
}

impl ProxyExt {
    pub fn new(service: Service, router: Arc<Router>) -> Self {
        Self {
            buf: Mutex::new(BytesMut::with_capacity(4096)),
            decoder: Mutex::new(Decoder::new()),
            service,
            router,
        }
    }
}

impl ProxyExt {
    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    fn indication(&self, message: MessageReader<'_, '_>, bytes: &mut BytesMut) -> Option<()> {
        let router = self.service.get_router();

        let data = message.get::<Data>()?;
        let peer = message.get::<XorPeerAddress>()?;
        let addr = router.get_port_bound(peer.port())?;
        let mark = router.get_node(&addr)?.mark;

        let mut pack = MessageWriter::extend(Method::DataIndication, &message, bytes);
        pack.append::<XorPeerAddress>(peer);
        pack.append::<Data>(data);
        pack.flush(None).ok()?;

        self.router.send(mark, StunClass::Message, &addr, &bytes);
        Some(())
    }

    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    fn channel(&self) {}
}

impl ProxyObserver for ProxyExt {
    /// Get user list.
    ///
    /// This interface returns the username and a list of addresses used by this
    /// user.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = Config::new()
    /// let service = Service::new(/* ... */);;
    /// let monitor = Monitor::new(/* ... */);
    ///
    /// let ctr = Controller::new(service.get_router(), config, monitor);
    /// // let users_js = ctr.get_users().await;
    /// ```
    fn relay(&self, payload: &[u8]) {
        if let (Ok(mut decoder), Ok(mut bytes)) = (self.decoder.lock(), self.buf.lock()) {
            if let Ok(payload) = decoder.decode(payload) {
                match payload {
                    Payload::Message(message) => {
                        self.indication(message, &mut bytes);
                    }
                    Payload::ChannelData(data) => {}
                }
            }
        }
    }
}
