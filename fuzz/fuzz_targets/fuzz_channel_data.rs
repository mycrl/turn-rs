#![no_main]

use libfuzzer_sys::fuzz_target;
use turn_server::codec::{Decoder, channel_data::ChannelData};
use bytes::BytesMut;

fuzz_target!(|data: &[u8]| {
    // Fuzz TURN ChannelData parsing and encoding
    let mut decoder = Decoder::default();
    
    // Try to decode the fuzzed data as channel data
    if let Ok(result) = decoder.decode(data) {
        if let Some(channel_data) = result.into_channel_data() {
            // Test channel data accessors
            let _ = channel_data.number();
            let _ = channel_data.bytes();
            
            // Test re-encoding for round-trip stability
            let mut buffer = BytesMut::with_capacity(1500);
            ChannelData::new(channel_data.number(), channel_data.bytes()).encode(&mut buffer);
        }
    }
    
    // Also test direct ChannelData construction with fuzzed input
    if data.len() >= 2 {
        let channel_number = u16::from_be_bytes([data[0], data[1]]);
        let payload = if data.len() > 2 { &data[2..] } else { &[] };
        
        let mut buffer = BytesMut::with_capacity(1500);
        ChannelData::new(channel_number, payload).encode(&mut buffer);
    }
});
