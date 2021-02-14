use anyhow::anyhow;
use std::convert::{
    TryFrom,
    Into,
};

/// ### RTP Header Extension
/// 
/// ```bash
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |      defined by profile       |           length              |
///  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  |                        header extension                       |
///  |                             ....                              |
/// ```
pub struct Extension {
    kind: u16,
    data: Vec<u32>,
}

impl<'a> TryFrom<'a [u8]> for Extension {
    type Error = anyhow::Error;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        
    }
}