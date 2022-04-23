mod format;
mod ffi;

use anyhow::Result;
pub use format::PixelFormat;

pub enum Task<T> {
    Ready(T),
    Wait,
    Eof,
}

pub struct EncoderBuilder {
    raw: ffi::EncoderOptions,
}

impl EncoderBuilder {
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
    pub fn new(name: &str) -> Result<Self> {
        let codec = ffi::safe_create_encoder(name)?;
        let mut raw = ffi::EncoderOptions::default();
        raw.codec = codec;
        Ok(Self { raw })
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
    pub fn set_width(&mut self, width: usize) -> &mut Self {
        self.raw.width = width as i32;
        self
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
    pub fn set_height(&mut self, height: usize) -> &mut Self {
        self.raw.height = height as i32;
        self
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
    pub fn set_bit_rate(&mut self, bit_rate: usize) -> &mut Self {
        self.raw.bit_rate = bit_rate as i64;
        self
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
    pub fn set_frame_rate(&mut self, frame_rate: usize) -> &mut Self {
        self.raw.frame_rate = frame_rate as i32;
        self
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
    pub fn set_format(&mut self, format: PixelFormat) -> &mut Self {
        self.raw.format = format as i32;
        self
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
    pub fn build(&self) -> Result<Encoder> {
        Encoder::new(&self.raw as *const ffi::EncoderOptions)
    }
}

pub struct Encoder {
    raw: *const ffi::Encoder,
    pub frame_size: usize,
}

impl Encoder {
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
    pub fn new(options: *const ffi::EncoderOptions) -> Result<Self> {
        let raw = ffi::safe_open_encoder(options)?;
        let frame_size = ffi::safe_encoder_get_buffer_size(raw)?;
        Ok(Self { frame_size, raw })
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
    pub fn write(&mut self, frame_buf: &[u8]) -> Result<Task<()>> {
        ffi::safe_encoder_write_frame(self.raw, frame_buf, self.frame_size)
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
    pub fn read<'a>(&'a mut self) -> Result<Task<&'a [u8]>> {
        Ok(match ffi::safe_encoder_receiver(self.raw)? {
            Task::Ready(()) => Task::Ready(ffi::safe_encoder_read::<'a>(self.raw)?),
            Task::Wait => Task::Wait,
            Task::Eof => Task::Eof,
        })
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
    pub fn flush(&mut self) {
        ffi::safe_encoder_clean(self.raw)
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        ffi::safe_encoder_free(self.raw);
        drop(self)
    }
}
