use bytes::BytesMut;
use std::sync::mpsc::Sender;
// use std::io::Error;
use crate::util::rand_strs;


/// # Handshake Info.
pub enum Handshake {
    Head, Body
}


/// # Create Handshake Package.
pub fn create_handshake (handshake: Handshake) -> Vec<u8> {
    match handshake {
        Handshake::Head => vec![3],
        Handshake::Body => {
            let mut head = vec![0; 8];
            head.extend_from_slice(&rand_strs(1528));
            head
        }
    }
}


/// # Examination Handshake Package.
pub fn is_handshake (bytes: &BytesMut) -> (bool, bool) {
    let mut is_type = false;
    let mut is_back = false;
    let mut index = 0;
    let mut offset = 0;

    // examination package length.
    // C0 + C1 || S0 + S1
    if bytes.len() == 1537 {
        // C0, S0
        // lock version number is 3
        if bytes[0] == 3 {
            index = 5;
            offset = 9;
            is_back = true;
        }
    } else {
        index = 4;
        offset = 8;
    }

    // C1, C2
    // S1, S2
    // TODO: check only the default placeholder.
    if index > 0 {
        if bytes[index] == 0 
        && bytes[index + 1] == 0 
        && bytes[index + 2] == 0 
        && bytes[index + 3] == 0 {
            if bytes.len() - offset == 1528 {
                is_type = true
            }
        }
    }

    // callback type and back.
    (is_type, is_back)
}


/// # Match Package.
pub fn match_package (bytes: BytesMut, sender: Sender<BytesMut>) {
    let (is_handshake_type, is_handshake_back) = is_handshake(&bytes);
    if is_handshake_type {
        if is_handshake_back {
            let mut package = create_handshake(Handshake::Head);
            package.extend_from_slice(&create_handshake(Handshake::Body));
            package.extend_from_slice(&create_handshake(Handshake::Body));
            let body = BytesMut::from(package);
            sender.send(body).unwrap();
        }
    }
}


/// # Decoder Bytes.
/// processing RTMP data.
/// 
pub fn decoder(bytes: BytesMut, sender: Sender<BytesMut>) {
    println!("{:?}", bytes.to_vec());
    println!("{:?}", bytes.len());
    match_package(bytes, sender);
}