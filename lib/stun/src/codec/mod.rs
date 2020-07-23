pub mod decode;

/// message type.
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Flag {
    BindingRequest = 0x0001,
}

/// message attributes.
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Attributes {
    Unknown = 0xC057,
    UserName = 0x0006,
    IceContrlling = 0x802A,
    UseCandidate = 0x0025,
    Priority = 0x0024,
    MessageIntegrity = 0x0008,
    Fingerprint = 0x8028,
    XorMappedAddress = 0x0020,
    MappedAddress = 0x0001,
    ResponseOrigin = 0x802B,
    Software = 0x8022,
}

/// message attribute.
#[derive(Clone, Debug)]
pub enum Attribute {
    UserName(String)
}
