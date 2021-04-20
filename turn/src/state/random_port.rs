use std::collections::HashMap;

const BUCKET_COUNT: u16 = 164;
const BUCKET_SIZE: u16 = 100;
const PORT_MIN: u16 = 49152;
const PORT_MAX: u16 = 65535;

pub struct RandomPort {
    buckets: HashMap<usize, u16>,
}