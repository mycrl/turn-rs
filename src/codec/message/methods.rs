use super::Error;

/// STUN Methods Registry
///
/// [RFC5389]: https://datatracker.ietf.org/doc/html/rfc5389
/// [RFC8489]: https://datatracker.ietf.org/doc/html/rfc8489
/// [RFC8126]: https://datatracker.ietf.org/doc/html/rfc8126
/// [Section 5]: https://datatracker.ietf.org/doc/html/rfc8489#section-5
///
/// A STUN method is a hex number in the range 0x000-0x0FF.  The encoding
/// of a STUN method into a STUN message is described in [Section 5].
///
/// STUN methods in the range 0x000-0x07F are assigned by IETF Review
/// [RFC8126].  STUN methods in the range 0x080-0x0FF are assigned by
/// Expert Review [RFC8126].  The responsibility of the expert is to
/// verify that the selected codepoint(s) is not in use and that the
/// request is not for an abnormally large number of codepoints.
/// Technical review of the extension itself is outside the scope of the
/// designated expert responsibility.
///
/// IANA has updated the name for method 0x002 as described below as well
/// as updated the reference from [RFC5389] to [RFC8489] for the following
/// STUN methods:
///
/// 0x000: Reserved
/// 0x001: Binding
/// 0x002: Reserved; was SharedSecret prior to [RFC5389]
/// 0x003: Allocate
/// 0x004: Refresh
/// 0x006: Send
/// 0x007: Data
/// 0x008: CreatePermission
/// 0x009: ChannelBind
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum MethodType {
    Request,
    Response,
    Error,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum Method {
    Binding(MethodType),
    Allocate(MethodType),
    CreatePermission(MethodType),
    ChannelBind(MethodType),
    Refresh(MethodType),
    SendIndication,
    DataIndication,
}

pub const BINDING_REQUEST: Method = Method::Binding(MethodType::Request);
pub const BINDING_RESPONSE: Method = Method::Binding(MethodType::Response);
pub const BINDING_ERROR: Method = Method::Binding(MethodType::Error);
pub const ALLOCATE_REQUEST: Method = Method::Allocate(MethodType::Request);
pub const ALLOCATE_RESPONSE: Method = Method::Allocate(MethodType::Response);
pub const ALLOCATE_ERROR: Method = Method::Allocate(MethodType::Error);
pub const CREATE_PERMISSION_REQUEST: Method = Method::CreatePermission(MethodType::Request);
pub const CREATE_PERMISSION_RESPONSE: Method = Method::CreatePermission(MethodType::Response);
pub const CREATE_PERMISSION_ERROR: Method = Method::CreatePermission(MethodType::Error);
pub const CHANNEL_BIND_REQUEST: Method = Method::ChannelBind(MethodType::Request);
pub const CHANNEL_BIND_RESPONSE: Method = Method::ChannelBind(MethodType::Response);
pub const CHANNEL_BIND_ERROR: Method = Method::ChannelBind(MethodType::Error);
pub const REFRESH_REQUEST: Method = Method::Refresh(MethodType::Request);
pub const REFRESH_RESPONSE: Method = Method::Refresh(MethodType::Response);
pub const REFRESH_ERROR: Method = Method::Refresh(MethodType::Error);
pub const SEND_INDICATION: Method = Method::SendIndication;
pub const DATA_INDICATION: Method = Method::DataIndication;

impl Method {
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Method::Binding(MethodType::Error)
                | Method::Refresh(MethodType::Error)
                | Method::Allocate(MethodType::Error)
                | Method::CreatePermission(MethodType::Error)
                | Method::ChannelBind(MethodType::Error)
        )
    }

    pub fn error(&self) -> Option<Method> {
        match self {
            Method::Binding(_) => Some(BINDING_ERROR),
            Method::Allocate(_) => Some(ALLOCATE_ERROR),
            Method::CreatePermission(_) => Some(CREATE_PERMISSION_ERROR),
            Method::ChannelBind(_) => Some(CHANNEL_BIND_ERROR),
            Method::Refresh(_) => Some(REFRESH_ERROR),
            _ => None,
        }
    }
}

impl TryFrom<u16> for Method {
    type Error = Error;

    /// # Test
    ///
    /// ```
    /// use turn_server_codec::message::methods::*;
    /// use std::convert::TryFrom;
    ///
    /// assert_eq!(Method::try_from(0x0001).unwrap(), BINDING_REQUEST);
    /// assert_eq!(Method::try_from(0x0101).unwrap(), BINDING_RESPONSE);
    /// assert_eq!(Method::try_from(0x0111).unwrap(), BINDING_ERROR);
    /// assert_eq!(Method::try_from(0x0003).unwrap(), ALLOCATE_REQUEST);
    /// assert_eq!(Method::try_from(0x0103).unwrap(), ALLOCATE_RESPONSE);
    /// assert_eq!(Method::try_from(0x0113).unwrap(), ALLOCATE_ERROR);
    /// assert_eq!(Method::try_from(0x0008).unwrap(), CREATE_PERMISSION_REQUEST);
    /// assert_eq!(Method::try_from(0x0108).unwrap(), CREATE_PERMISSION_RESPONSE);
    /// assert_eq!(Method::try_from(0x0118).unwrap(), CREATE_PERMISSION_ERROR);
    /// assert_eq!(Method::try_from(0x0009).unwrap(), CHANNEL_BIND_REQUEST);
    /// assert_eq!(Method::try_from(0x0109).unwrap(), CHANNEL_BIND_RESPONSE);
    /// assert_eq!(Method::try_from(0x0119).unwrap(), CHANNEL_BIND_ERROR);
    /// assert_eq!(Method::try_from(0x0004).unwrap(), REFRESH_REQUEST);
    /// assert_eq!(Method::try_from(0x0104).unwrap(), REFRESH_RESPONSE);
    /// assert_eq!(Method::try_from(0x0114).unwrap(), REFRESH_ERROR);
    /// assert_eq!(Method::try_from(0x0016).unwrap(), SEND_INDICATION);
    /// assert_eq!(Method::try_from(0x0017).unwrap(), DATA_INDICATION);
    /// ```
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok(match value {
            0x0001 => Self::Binding(MethodType::Request),
            0x0101 => Self::Binding(MethodType::Response),
            0x0111 => Self::Binding(MethodType::Error),
            0x0003 => Self::Allocate(MethodType::Request),
            0x0103 => Self::Allocate(MethodType::Response),
            0x0113 => Self::Allocate(MethodType::Error),
            0x0008 => Self::CreatePermission(MethodType::Request),
            0x0108 => Self::CreatePermission(MethodType::Response),
            0x0118 => Self::CreatePermission(MethodType::Error),
            0x0009 => Self::ChannelBind(MethodType::Request),
            0x0109 => Self::ChannelBind(MethodType::Response),
            0x0119 => Self::ChannelBind(MethodType::Error),
            0x0004 => Self::Refresh(MethodType::Request),
            0x0104 => Self::Refresh(MethodType::Response),
            0x0114 => Self::Refresh(MethodType::Error),
            0x0016 => Self::SendIndication,
            0x0017 => Self::DataIndication,
            _ => return Err(Error::UnknownMethod),
        })
    }
}

impl From<Method> for u16 {
    /// # Test
    ///
    /// ```
    /// use turn_server_codec::message::methods::*;
    /// use std::convert::From;
    ///
    /// assert_eq!(0x0001u16, u16::from(BINDING_REQUEST));
    /// assert_eq!(0x0101u16, u16::from(BINDING_RESPONSE));
    /// assert_eq!(0x0111u16, u16::from(BINDING_ERROR));
    /// assert_eq!(0x0003u16, u16::from(ALLOCATE_REQUEST));
    /// assert_eq!(0x0103u16, u16::from(ALLOCATE_RESPONSE));
    /// assert_eq!(0x0113u16, u16::from(ALLOCATE_ERROR));
    /// assert_eq!(0x0008u16, u16::from(CREATE_PERMISSION_REQUEST));
    /// assert_eq!(0x0108u16, u16::from(CREATE_PERMISSION_RESPONSE));
    /// assert_eq!(0x0118u16, u16::from(CREATE_PERMISSION_ERROR));
    /// assert_eq!(0x0009u16, u16::from(CHANNEL_BIND_REQUEST));
    /// assert_eq!(0x0109u16, u16::from(CHANNEL_BIND_RESPONSE));
    /// assert_eq!(0x0119u16, u16::from(CHANNEL_BIND_ERROR));
    /// assert_eq!(0x0004u16, u16::from(REFRESH_REQUEST));
    /// assert_eq!(0x0104u16, u16::from(REFRESH_RESPONSE));
    /// assert_eq!(0x0114u16, u16::from(REFRESH_ERROR));
    /// assert_eq!(0x0016u16, u16::from(SEND_INDICATION));
    /// assert_eq!(0x0017u16, u16::from(DATA_INDICATION));
    /// ```
    fn from(val: Method) -> Self {
        match val {
            Method::Binding(MethodType::Request) => 0x0001,
            Method::Binding(MethodType::Response) => 0x0101,
            Method::Binding(MethodType::Error) => 0x0111,
            Method::Allocate(MethodType::Request) => 0x0003,
            Method::Allocate(MethodType::Response) => 0x0103,
            Method::Allocate(MethodType::Error) => 0x0113,
            Method::CreatePermission(MethodType::Request) => 0x0008,
            Method::CreatePermission(MethodType::Response) => 0x0108,
            Method::CreatePermission(MethodType::Error) => 0x0118,
            Method::ChannelBind(MethodType::Request) => 0x0009,
            Method::ChannelBind(MethodType::Response) => 0x0109,
            Method::ChannelBind(MethodType::Error) => 0x0119,
            Method::Refresh(MethodType::Request) => 0x0004,
            Method::Refresh(MethodType::Response) => 0x0104,
            Method::Refresh(MethodType::Error) => 0x0114,
            Method::SendIndication => 0x0016,
            Method::DataIndication => 0x0017,
        }
    }
}
