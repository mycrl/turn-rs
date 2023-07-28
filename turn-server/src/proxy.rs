use crate::router::Router;

use std::sync::Arc;

use bytes::BytesMut;
use faster_stun::attribute::*;
use faster_stun::*;
use turn_proxy::ProxyObserver;
use turn_rs::{Service, StunClass};

pub struct ProxyExt {
    service: Service,
    router: Arc<Router>,
    decoder: Decoder,
    buf: BytesMut,
}

impl ProxyExt {
    pub fn new(service: Service, router: Arc<Router>) -> Self {
        Self {
            buf: BytesMut::with_capacity(4096),
            decoder: Decoder::new(),
            service,
            router,
        }
    }
}

impl ProxyExt {
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

    fn channel(&self) {}
}

impl ProxyObserver for ProxyExt {
    fn relay(&mut self, payload: &[u8]) {
        if let Ok(payload) = self.decoder.decode(payload) {
            match payload {
                Payload::Message(message) => {
                    // self.indication(message, &mut self.buf);
                }
                Payload::ChannelData(data) => {}
            }
        }
    }
}
