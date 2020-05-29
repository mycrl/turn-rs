//! Exponential-Golomb coding
//!
//! An exponential-Golomb code (or just Exp-Golomb code) is a type of universal code.
//! To encode any nonnegative integer x using the exp-Golomb code:
//!     1. Write down x+1 in binary
//!     2. Count the bits written, subtract one, and write that number of starting zero bits preceding the previous bit string.
//! The first few values of the code are:
//!
//! ```
//! 0 ⇒ 1 ⇒ 1
//! 1 ⇒ 10 ⇒ 010
//! 2 ⇒ 11 ⇒ 011
//! 3 ⇒ 100 ⇒ 00100
//! 4 ⇒ 101 ⇒ 00101
//! 5 ⇒ 110 ⇒ 00110
//! 6 ⇒ 111 ⇒ 00111
//! 7 ⇒ 1000 ⇒ 0001000
//! 8 ⇒ 1001 ⇒ 0001001
//! ...[1]
//! ```

use std::cmp;

/// Exponential-Golomb coding
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
    /// 创建哥伦布编码实例
    ///
    /// 传入缓冲器，
    /// 以ExpGolomb编码方式读取.
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

    /// 填充当前位
    #[allow(bad_style)]
    fn fill_current_word(&mut self) {
        let bufferBytesLeft = self.totalBytes - self.bufferIndex;
        let bytesRead = cmp::min(4, bufferBytesLeft);
        self.currentWord = {
            let mut buffer = [0u8; 4];
            (&mut buffer[0..bytesRead])
                .copy_from_slice(&self.buffer[self.bufferIndex..(self.bufferIndex + bytesRead)]);
            u32::from_be_bytes(buffer)
        };
        self.bufferIndex += bytesRead;
        self.currentWordBitsLeft = bytesRead * 8;
    }

    /// 读取指定bit位
    pub fn read_bits(&mut self, bits: usize) -> u32 {
        if bits <= self.currentWordBitsLeft {
            let result = self.currentWord >> (32 - bits);
            self.currentWord <<= bits;
            self.currentWordBitsLeft -= bits;
            return result;
        }

        let mut result = if self.currentWordBitsLeft > 0 {
            self.currentWord
        } else {
            0
        };

        result = if self.currentWordBitsLeft > 0 {
            result >> (32 - self.currentWordBitsLeft)
        } else {
            0
        };

        let bits_need_left = bits - self.currentWordBitsLeft;
        self.fill_current_word();

        let bits_read_next = cmp::min(bits_need_left, self.currentWordBitsLeft);
        let result2 = self.currentWord >> (32 - bits_read_next);
        self.currentWord <<= bits_read_next;
        self.currentWordBitsLeft -= bits_read_next;
        result = (result << bits_read_next) | result2;
        result
    }

    /// 读取布尔值
    pub fn read_bool(&mut self) -> bool {
        self.read_bits(1) == 1
    }

    /// 读取字节
    pub fn read_byte(&mut self) -> u8 {
        self.read_bits(8) as u8
    }

    /// 跳过前置零位
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

    /// unsigned exponential golomb
    pub fn read_ueg(&mut self) -> u32 {
        let leading_zeros = self.skip_leading_zero();
        self.read_bits(leading_zeros + 1) - 1
    }

    /// signed exponential golomb
    pub fn read_seg(&mut self) -> i32 {
        let value = self.read_ueg();
        match value & 0x01 != 0 {
            true => ((value + 1) >> 1) as i32,
            false => -((value >> 1) as i32),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ExpGolomb;

    #[test]
    fn it_works() {
        let mut gb = ExpGolomb::new(&[0, 1, 2, 3, 4, 5]);
        assert_eq!(gb.read_byte(), 0);
        assert_eq!(gb.read_bits(8), 1);
        assert_eq!(gb.read_byte(), 2);
        assert_eq!(gb.read_bool(), false);
        assert_eq!(gb.read_seg(), 24);
        assert_eq!(gb.read_ueg(), 1);
    }
}
