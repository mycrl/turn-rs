use std::net::SocketAddr;

use bytes::BytesMut;
use codec::{
    crypto::Password,
    message::{
        Message,
        attributes::{PasswordAlgorithm, UserName},
    },
};

use crate::{
    ServiceHandler,
    routing::{State, response::ResponseTarget},
    session::{Endpoint, Identifier},
};

pub(crate) struct Request<'a, 'b, T, M>
where
    T: ServiceHandler,
{
    pub id: &'a Identifier,
    pub encode_buffer: &'b mut BytesMut,
    pub state: &'a State<T>,
    pub payload: &'a M,
}

impl<'a, 'b, T, M> Request<'a, 'b, T, M>
where
    T: ServiceHandler,
{
    #[inline(always)]
    pub(crate) fn target(&self, endpoint: Endpoint) -> ResponseTarget {
        ResponseTarget {
            relay: Some(endpoint.source),
            endpoint: if self.state.endpoint != endpoint.endpoint {
                Some(endpoint.endpoint)
            } else {
                None
            },
        }
    }
}

impl<'a, 'b, T> Request<'a, 'b, T, Message<'a>>
where
    T: ServiceHandler,
{
    // Verify the IP address specified by the client in the request, such as the
    // peer address used when creating permissions and binding channels. Currently,
    // only peer addresses that are local addresses of the TURN server are allowed;
    // arbitrary addresses are not permitted.
    //
    // Allowing arbitrary addresses would pose security risks, such as enabling
    // the TURN server to forward data to any target.
    #[inline(always)]
    pub fn verify_ip(&self, address: &SocketAddr) -> bool {
        self.state
            .interfaces
            .iter()
            .any(|item| item.ip() == address.ip())
    }

    // Verify the integrity of the request message.
    #[inline(always)]
    pub async fn verify(&self) -> Option<(&str, Password)> {
        let username = self.payload.get::<UserName>()?;
        let algorithm = self
            .payload
            .get::<PasswordAlgorithm>()
            .unwrap_or(PasswordAlgorithm::Md5);

        let password = self
            .state
            .manager
            .get_password(self.id, username, algorithm)
            .await?;

        if self.payload.verify(&password).is_err() {
            return None;
        }

        Some((username, password))
    }
}
