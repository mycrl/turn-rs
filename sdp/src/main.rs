use std::convert::TryFrom;
use sdp::*;

fn main() {
    let sdp_str = std::fs::read_to_string("../workflow/offer.sdp").unwrap();
    println!("{:#?}", Sdp::try_from(sdp_str.as_str()).unwrap());
}
