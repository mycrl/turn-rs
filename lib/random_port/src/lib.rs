#![feature(asm)]

pub struct Buckets {
    inner: [u64; 256]
}

impl Buckets {
    pub fn new() -> Self {
        Self { inner: [0u64; 256] }
    }
    
    fn find(&self, index: usize) -> Option<usize> {
        let value = self.inner[index];
        if value == u64::MAX {
            return None
        }
        
        let index: u64;
        unsafe { asm!("bsr {}, {}", out(reg) index, in(reg) value) }
        (63 - index) as usize
    }
}
