#![no_main]

use libfuzzer_sys::fuzz_target;
use turn_server::codec::Decoder;

fuzz_target!(|data: &[u8]| {
    // Fuzz the STUN/TURN message decoder
    // This tests the robustness of the codec against malformed inputs
    let mut decoder = Decoder::default();
    let _ = decoder.decode(data);
});
