use bytes::{BytesMut, Buf};
use std::cmp;

pub struct ExpGolomb {
    buffer: BytesMut,
    index: usize,
    total_bits: usize,
    total_bytes: usize,
    current_word: usize,
    current_word_bits_left: usize
}

impl ExpGolomb {
    pub fn new(data: &[u8]) -> Self {
        let total_bytes = data.len();
        Self {
            index: 0,
            total_bytes,
            current_word: 0,
            current_word_bits_left: 0,
            total_bits: total_bytes * 8,
            buffer: BytesMut::from(data),
        }
    }

    #[rustfmt::skip]
    fn fill_current_word(&mut self) {
        let buffer_bytes_left = self.total_bytes - self.index;
        let bytes_read = cmp::min(4, buffer_bytes_left);
        let mut word = BytesMut::from(&self.buffer[self.index..self.index + bytes_read]);
        self.current_word = word.get_u32() as usize;
        self.current_word_bits_left = bytes_read * 8;
        self.index = bytes_read;
    }

    #[rustfmt::skip]
    fn skip_leading_zero(&mut self) -> usize {
        let mut zero_count = 0;
        for _ in zero_count..self.current_word_bits_left {
            if self.current_word & (0x80000000 >> zero_count) != 0 {
                self.current_word <<= zero_count;
                self.current_word_bits_left -= zero_count;
                return zero_count;
            }

            zero_count += 1;
        }

        self.fill_current_word();
        return zero_count + self.skip_leading_zero();
    }

    #[rustfmt::skip]
    pub fn read_bits(&mut self, bits: usize) -> usize {
        if bits <= self.current_word_bits_left {
            let result = self.current_word >> (32 - bits);
            self.current_word_bits_left -= bits;
            self.current_word <<= bits;
            return result;
        }

        let mut result = if self.current_word_bits_left > 0 { self.current_word } else { 0 };
        result = result >> (32 - self.current_word_bits_left);
        let bits_need_left = bits - self.current_word_bits_left;
        self.fill_current_word();
        let bits_read_next = cmp::min(bits_need_left, self.current_word_bits_left);
        let result2 = self.current_word >> (32 - bits_read_next);
        self.current_word_bits_left -= bits_read_next;
        self.current_word <<= bits_read_next;
        result = (result << bits_read_next) | result2;
        result
    }

    #[rustfmt::skip]
    pub fn read_bool(&mut self) -> bool {
        self.read_bits(1) == 1
    }

    #[rustfmt::skip]
    pub fn read_byte(&mut self) -> usize {
        self.read_bits(8)
    }

    #[rustfmt::skip]
    pub fn read_ueg(&mut self) -> usize {
        let leading_zeros = self.skip_leading_zero();
        self.read_bits(leading_zeros + 1) - 1
    }

    #[rustfmt::skip]
    pub fn read_seg(&mut self) -> usize {
        let value = self.read_ueg();
        match value & 0x01 > 0 {
            true => (value + 1) >> 1,
            false => 0 - (value >> 1)
        }
    }
}
