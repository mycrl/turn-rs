

pub struct Bitmap(u64);

impl Bitmap {
    pub fn first(&mut self, bit: u8) -> Option<usize> {
        let full = match bit { 
            0 => u64::MIN,
            _ => u64::MAX
        };
        
        if self.0 == full {
            return None
        }

        None
    }
}
