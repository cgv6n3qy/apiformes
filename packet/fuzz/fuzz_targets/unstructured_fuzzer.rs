#![no_main]
use apiformes_packet::prelude::*;
use bytes::Bytes;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut bytes = Bytes::copy_from_slice(data);
    let _ = Packet::from_bytes(&mut bytes);
});
