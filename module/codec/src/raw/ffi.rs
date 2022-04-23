use super::{
    format::PixelFormat,
    Task
};

use std::{
    slice::from_raw_parts,
    ffi::CString,
};

use libc::{
    c_void,
    c_int,
    c_char,
};

use anyhow::{
    Result,
    ensure,
    anyhow,
};

#[repr(C)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum CodecStatus {
    RR_ERROR,
    RR_Ready,
	RR_Wait,
	RR_Eof
}

impl CodecStatus {
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer: [u8; 4] = [
    ///     0x00, 0x01, 0x00, 0x00
    /// ];
    ///         
    /// let data = ChannelData::try_from(&buffer[..]).unwrap();
    /// assert_eq!(data.number, 1);
    /// ```
    fn to_task(self, msg: &'static str) -> Result<Task<()>> {
        match self {
            Self::RR_ERROR => Err(anyhow!(msg)),
            Self::RR_Ready => Ok(Task::Ready(())),
            Self::RR_Wait => Ok(Task::Wait),
            Self::RR_Eof => Ok(Task::Eof),
        }
    }
}

#[repr(C)]
pub struct EncoderOptions {
    pub codec: *const c_void,
    pub width: i32,
    pub height: i32,
    pub bit_rate: i64,
    pub frame_rate: i32,
    pub format: c_int,
}

impl Default for EncoderOptions {
    /// # Unit Test
    ///
    /// ```
    /// use stun::*;
    /// use std::convert::TryFrom;
    /// 
    /// let buffer: [u8; 4] = [
    ///     0x00, 0x01, 0x00, 0x00
    /// ];
    ///         
    /// let data = ChannelData::try_from(&buffer[..]).unwrap();
    /// assert_eq!(data.number, 1);
    /// ```
    fn default() -> Self {
        Self {
            format: PixelFormat::YUV420P as c_int,
            codec: std::ptr::null(),
            bit_rate: 400000,
            frame_rate: 25,
            width: 1280,
            height: 720, 
        }
    }
}

#[repr(C)]
pub struct Encoder {
    options: *const EncoderOptions,
    ctx: *const c_void,
    packet: *const c_void,
    frame: *const c_void,
    pts: i64,
}

#[repr(C)]
pub struct Chunk {
    data: *const u8,
	len: c_int,
}

#[link(name = "VideoCodec", kind = "dylib")]
extern "C" {
    fn create_encoder(name: *const c_char) -> *const c_void;
    fn open_encoder(options: *const EncoderOptions) -> *const Encoder;
    fn encoder_get_buffer_size(encoder: *const Encoder) -> i32;
    fn encoder_write_frame(encoder: *const Encoder, buf: *const u8, frame_bytes: c_int) -> CodecStatus;
    fn encoder_receiver(encoder: *const Encoder) -> CodecStatus;
    fn encoder_get_pkt_chunk(encoder: *const Encoder) -> Chunk;
    fn encoder_clean(encoder: *const Encoder);
    fn encoder_free(encoder: *const Encoder);
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_create_encoder(name: &str) -> Result<*const c_void> {
    let c_name = CString::new(name).unwrap();
    let codec = unsafe { create_encoder(c_name.as_ptr()) };
    ensure!(!codec.is_null(), "create encoder faild!");
    Ok(codec)
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_open_encoder(options: *const EncoderOptions) -> Result<*const Encoder> {
    let encoder = unsafe { open_encoder(options) };
    ensure!(!encoder.is_null(), "open encoder faild!");
    Ok(encoder)
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_encoder_get_buffer_size(raw: *const Encoder) -> Result<usize> {
    let frame_size = unsafe { encoder_get_buffer_size(raw) as usize };
    ensure!(frame_size > 0, "get frame size faild!");
    Ok(frame_size)
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_encoder_write_frame(raw: *const Encoder, frame_buf: &[u8], frame_size: usize) -> Result<Task<()>> {
    let status = unsafe { encoder_write_frame(raw, frame_buf.as_ptr(), frame_size as i32) };
    status.to_task("write frame error!")
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_encoder_receiver(raw: *const Encoder) -> Result<Task<()>> {
    let status = unsafe { encoder_receiver(raw) };
    status.to_task("receiver error!")
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_encoder_read<'a>(raw: *const Encoder) -> Result<&'a [u8]> {
    let chunk = unsafe { encoder_get_pkt_chunk(raw) };
    ensure!(!chunk.data.is_null(), "get packet data faild!");
    Ok(unsafe { from_raw_parts(chunk.data, chunk.len as usize) })
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_encoder_clean(raw: *const Encoder) {
    unsafe { encoder_clean(raw) }
}

/// # Unit Test
///
/// ```
/// use stun::*;
/// use std::convert::TryFrom;
/// 
/// let buffer: [u8; 4] = [
///     0x00, 0x01, 0x00, 0x00
/// ];
///         
/// let data = ChannelData::try_from(&buffer[..]).unwrap();
/// assert_eq!(data.number, 1);
/// ```
pub fn safe_encoder_free(raw: *const Encoder) {
    unsafe { encoder_free(raw) }
}
