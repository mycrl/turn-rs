use std::net::SocketAddr;

use bytes::BytesMut;
use stun::{
    attribute::{
        ErrKind::{AllocationMismatch, ServerError, Unauthorized},
        Lifetime, Nonce, ReqeestedTransport, Software, XorMappedAddress, XorRelayedAddress,
    },
    MessageReader,
};

use super::{MessageRouter, Requet, Response, RouterError};
use crate::{ensure, ensure_optional, Observer, SOFTWARE};

/// [rfc8489](https://tools.ietf.org/html/rfc8489)
///
/// In all cases, the server SHOULD only allocate ports from the range
/// 49152 - 65535 (the Dynamic and/or Private Port range [PORT-NUMBERS]),
/// unless the TURN server application knows, through some means not
/// specified here, that other applications running on the same host as
/// the TURN server application will not be impacted by allocating ports
/// outside this range.  This condition can often be satisfied by running
/// the TURN server application on a dedicated machine and/or by
/// arranging that any other applications on the machine allocate ports
/// before the TURN server application starts.  In any case, the TURN
/// server SHOULD NOT allocate ports in the range 0 - 1023 (the Well-
/// Known Port range) to discourage clients from using TURN to run
/// standard services.
pub(crate) struct Allocate;

impl<'a, T> MessageRouter<'a, T> for Allocate
where
    T: Observer + 'static,
{
    const AUTH: bool = true;

    #[rustfmt::skip]
    fn handle(bytes: &'a mut BytesMut, req: Requet<'a, T, MessageReader<'a, 'a>>) -> Result<Option<Response<'a>>, RouterError> {
        ensure!(!req.service.state.is_port_allcated(&req.address), ServerError);
        ensure!(req.message.get::<ReqeestedTransport>().is_none(), AllocationMismatch);

        let auth = ensure_optional!(&req.auth, Unauthorized);
        let port = ensure_optional!(req.service.state.alloc_port(&req.address), AllocationMismatch);
        req.service.observer.allocated(&req.address, auth.username, port);

        let mut message = req.create_message(bytes)?;
        message.append::<Nonce>(req.service.state.get_nonce(&req.address).as_str());
        message.append::<XorRelayedAddress>(SocketAddr::new(req.service.external.ip(), port));
        message.append::<XorMappedAddress>(req.address);
        message.append::<Lifetime>(600);
        message.append::<Software>(SOFTWARE);

        Ok(Some(req.create_response(message)?))
    }
}
