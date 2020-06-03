//! Exponential-Golomb coding
//!
//! @author Potato_TooLarge@telegram
//! @author Mr.Panda <xivistudios@gmail.com>

use std::cmp;

/// Exponential-Golomb coding
/// An exponential-Golomb code (or just Exp-Golomb code) is a type of universal code.
/// To encode any nonnegative integer x using the exp-Golomb code:
/// 1. Write down x+1 in binary
/// 2. Count the bits written, subtract one, and write that 
/// number of starting zero bits preceding the previous 
/// bit string.
#[derive(Debug)]
#[allow(bad_style)]
pub struct ExpGolomb<'a> {
    buffer: &'a [u8],
    bufferIndex: usize,
    totalBytes: usize,
    totalBits: usize,
    currentWord: u32,
    currentWordBitsLeft: usize,
}

impl<'a> ExpGolomb<'a> {
    /// Create Exponential-Golomb coding instance
    ///
    /// Incoming buffer,
    /// Read in ExpGolomb encoding.
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use golomb::ExpGolomb;
    ///
    /// ExpGolomb::new(&[0, 1, 2, 3, 4, 5])
    /// ```
    pub fn new(data: &'a [u8]) -> Self {
        let size = data.len();
        Self {
            buffer: data,
            bufferIndex: 0,
            totalBytes: size,
            totalBits: size * 8,
            currentWord: 0,
            currentWordBitsLeft: 0,
        }
    }

    /// 读取指定bit位
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use golomb::ExpGolomb;
    ///
    /// let mut gb = ExpGolomb::new(&[0, 1, 2, 3, 4, 5]);
    /// assert_eq!(gb.read_bits(8), 1);
    /// ```
    pub fn read_bits(&mut self, bits: usize) -> u32 {
        if bits <= self.currentWordBitsLeft {
            let result = self.currentWord >> (32 - bits);
            self.currentWord <<= bits;
            self.currentWordBitsLeft -= bits;
            return result;
        }

        let result = if self.currentWordBitsLeft > 0 {
            self.currentWord >> (32 - self.currentWordBitsLeft)
        } else {
            0
        };

        let bits_need_left = bits - self.currentWordBitsLeft;
        self.fill_current_word();

        let bits_read_next = cmp::min(bits_need_left, self.currentWordBitsLeft);
        let result2 = self.currentWord >> (32 - bits_read_next);
        self.currentWord <<= bits_read_next;
        self.currentWordBitsLeft -= bits_read_next;
        (result << bits_read_next) | result2
    }

    /// 读取布尔值
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use golomb::ExpGolomb;
    ///
    /// let mut gb = ExpGolomb::new(&[0, 1, 2, 3, 4, 5]);
    /// assert_eq!(gb.read_bool(), false);
    /// ```
    pub fn read_bool(&mut self) -> bool {
        self.read_bits(1) == 1
    }

    /// 读取字节
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use golomb::ExpGolomb;
    ///
    /// let mut gb = ExpGolomb::new(&[0, 1, 2, 3, 4, 5]);
    /// assert_eq!(gb.read_byte(), 0);
    /// ```
    pub fn read_byte(&mut self) -> u8 {
        self.read_bits(8) as u8
    }

    /// unsigned exponential golomb
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use golomb::ExpGolomb;
    ///
    /// let mut gb = ExpGolomb::new(&[0, 1, 2, 3, 4, 5]);
    /// assert_eq!(gb.read_seg(), 24);
    /// ```
    pub fn read_ueg(&mut self) -> u32 {
        let leading_zeros = self.skip_leading_zero();
        self.read_bits(leading_zeros + 1) - 1
    }

    /// signed exponential golomb
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use golomb::ExpGolomb;
    ///
    /// let mut gb = ExpGolomb::new(&[0, 1, 2, 3, 4, 5]);
    /// assert_eq!(gb.read_ueg(), 1);
    /// ``
    pub fn read_seg(&mut self) -> i32 {
        let value = self.read_ueg();
        match value & 0x01 != 0 {
            true => ((value + 1) >> 1) as i32,
            false => -((value >> 1) as i32),
        }
    }

    /// Fill in current bit
    /// 
    /// Encode ⌊x/2k⌋ using order-0 exp-Golomb code described above, then
    /// Encode x mod 2k in binary.
    #[allow(bad_style)]
    fn fill_current_word(&mut self) {
        let bufferBytesLeft = self.totalBytes - self.bufferIndex;
        if bufferBytesLeft == 0 { return; }
        let bytesRead = cmp::min(4, bufferBytesLeft);
        let mut buffer = [0u8; 4];
        let end_index = self.bufferIndex + bytesRead;
        let chunk = &self.buffer[self.bufferIndex..end_index];
        (&mut buffer[0..bytesRead]).copy_from_slice(chunk);
        self.currentWord = u32::from_be_bytes(buffer);
        self.bufferIndex += bytesRead;
        self.currentWordBitsLeft = bytesRead * 8;
    }

    /// Skip the leading zero
    /// 
    /// Delete k leading zero bits from the encoding result.
    #[allow(bad_style)]
    fn skip_leading_zero(&mut self) -> usize {
        let mut zero_count = 0;
        for i in 0..self.currentWordBitsLeft {
            zero_count = i;
            if self.currentWord & (0x80000000u32 >> i) != 0 {
                self.currentWordBitsLeft -= i;
                self.currentWord <<= i;
                return zero_count as usize;
            }
        }

        self.fill_current_word();
        zero_count as usize + self.skip_leading_zero()
    }
}
