#![no_main]

use libfuzzer_sys::fuzz_target;
use turn_server::codec::{
    Decoder,
    message::attributes::*,
};

fuzz_target!(|data: &[u8]| {
    // Fuzz STUN attribute parsing
    let mut decoder = Decoder::default();
    
    if let Ok(result) = decoder.decode(data) {
        if let Some(message) = result.into_message() {
            // Try to extract various STUN/TURN attributes
            // This tests attribute parsing robustness
            
            // Address attributes
            let _ = message.get::<MappedAddress>();
            let _ = message.get::<XorMappedAddress>();
            let _ = message.get::<XorRelayedAddress>();
            let _ = message.get::<XorPeerAddress>();
            let _ = message.get::<ResponseOrigin>();
            
            // String attributes
            let _ = message.get::<UserName>();
            let _ = message.get::<Realm>();
            let _ = message.get::<Nonce>();
            let _ = message.get::<Software>();
            
            // Numeric attributes
            let _ = message.get::<Lifetime>();
            let _ = message.get::<ChannelNumber>();
            let _ = message.get::<Fingerprint>();
            
            // Enum attributes
            let _ = message.get::<ReqeestedTransport>();
            let _ = message.get::<ErrorCode>();
            
            // Complex attributes
            let _ = message.get::<Data>();
            let _ = message.get::<MessageIntegrity>();
            let _ = message.get::<MessageIntegritySha256>();
            let _ = message.get::<PasswordAlgorithms>();
            let _ = message.get::<PasswordAlgorithm>();
            let _ = message.get::<UserHash>();
        }
    }
});
