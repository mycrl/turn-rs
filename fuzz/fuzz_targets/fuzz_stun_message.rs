#![no_main]

use libfuzzer_sys::fuzz_target;
use turn_server::codec::Decoder;

fuzz_target!(|data: &[u8]| {
    // Fuzz STUN message parsing and encoding
    let mut decoder = Decoder::default();
    
    // Try to decode the fuzzed data
    if let Ok(result) = decoder.decode(data) {
        if let Some(message) = result.into_message() {
            // Try to access various message properties without panicking
            let _ = message.method();
            let _ = message.transaction_id();
            
            // Test message integrity verification with random passwords
            use turn_server::codec::crypto::Password;
            let test_password = Password::Md5([0u8; 16]);
            let _ = message.verify(&test_password);
        }
    }
});
