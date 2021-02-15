use anyhow::ensure;
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
#[derive(Debug, Clone)]
pub struct Extension {
    pub kind: u16,
    pub data: Vec<u32>,
}

impl<'a> TryFrom<&'a [u8]> for Extension {
    type Error = anyhow::Error;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        ensure!(buf.len() >= 4, "buf len < 4");
        
        // get extension type and body size.
        let kind = convert::as_u16(&buf[0..2]);
        let size = convert::as_u16(&buf[2..4]);
        
        // get extension list.
        let mut data = Vec::new();
        for i in 0..(size as usize) {
            data.push(convert::as_u32(
                &buf[4 + (i * 4)..]
            ));
        }
        
        Ok(Self {
            kind,
            data
        })
    }
}