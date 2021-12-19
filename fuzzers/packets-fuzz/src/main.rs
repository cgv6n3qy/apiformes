#[macro_use]
extern crate afl;
use apiformes::packets::prelude::*;
use bytes::Bytes;
fn main() {
    fuzz!(|data: &[u8]| {
        // It requires more investigation into why only
        // very few paths are explored
        // one idea would be to bypass the initial packet check by hard coding
        // the fixed packet header before whatever afl throws into us
        let mut bytes = Bytes::copy_from_slice(data);
        let _ = Packet::from_bytes(&mut bytes);
    })
}
