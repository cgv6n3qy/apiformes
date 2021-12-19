pub fn bits_u8(data: u8, start: usize, len: usize) -> u8 {
    (data >> start) & ((2 << (len - 1)) - 1)
}
