// Data representation for MQTT v5.0 as per section 1.5

use super::{error::DataParseError, parsable::*};
use bytes::{Buf, BufMut, Bytes};
#[cfg(feature = "debug")]
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct MqttOneBytesInt(u8);
impl MqttOneBytesInt {
    pub(super) fn new(i: u8) -> Self {
        MqttOneBytesInt(i)
    }
    pub(super) fn inner(&self) -> u8 {
        self.0
    }
}

impl MqttUncheckedDeserialize for MqttOneBytesInt {
    fn fixed_size() -> usize {
        1
    }
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Ok(MqttOneBytesInt(buf.get_u8()))
    }
}

impl MqttSerialize for MqttOneBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(self.0)
    }
}

/// 1.5.2 Two Byte Integer
#[derive(Clone)]
pub(super) struct MqttTwoBytesInt(u16);

impl MqttTwoBytesInt {
    pub(super) fn new(i: u16) -> Self {
        MqttTwoBytesInt(i)
    }
    pub(super) fn inner(&self) -> u16 {
        self.0
    }
}

impl MqttUncheckedDeserialize for MqttTwoBytesInt {
    fn fixed_size() -> usize {
        2
    }
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Ok(MqttTwoBytesInt(buf.get_u16()))
    }
}

impl MqttSerialize for MqttTwoBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.0)
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttTwoBytesInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "0x{:04x}", self.0)
    }
}

/// 1.5.3 Four Byte Integer
#[derive(Clone)]
pub(super) struct MqttFourBytesInt(u32);

impl MqttFourBytesInt {
    pub(super) fn new(i: u32) -> Self {
        MqttFourBytesInt(i)
    }
    pub(super) fn inner(&self) -> u32 {
        self.0
    }
}

impl MqttUncheckedDeserialize for MqttFourBytesInt {
    fn fixed_size() -> usize {
        4
    }
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Ok(MqttFourBytesInt(buf.get_u32()))
    }
}

impl MqttSerialize for MqttFourBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u32(self.0)
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttFourBytesInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "0x{:08x}", self.0)
    }
}

/// 1.5.4 UTF-8 Encoded String
#[derive(Clone)]
pub(super) struct MqttUtf8String {
    s: Arc<str>,
}

impl MqttUtf8String {
    pub(super) fn new(s: Arc<str>) -> Result<Self, DataParseError> {
        MqttUtf8String::verify(&s)?;
        Ok(MqttUtf8String { s })
    }

    pub(super) fn unwrap(self) -> Arc<str> {
        self.s
    }

    pub(super) fn inner(&self) -> &Arc<str> {
        &self.s
    }

    /// TODO check if this verify is actually correct
    /// According to the specs:
    /// the maximum size of a UTF-8 Encoded String is 65,535 bytes.
    /// The character data in a UTF-8 Encoded String MUST be well-formed UTF-8 as
    /// defined by the Unicode specification and restated in RFC 3629. In particular,
    /// the character data MUST NOT include encodings of code points between U+D800
    /// and U+DFFF. A UTF-8 Encoded String MUST NOT include an encoding of the null
    /// character U+0000. The data SHOULD NOT include encodings of the Unicode code
    /// points listed below.
    /// - U+0001..U+001F control characters
    /// - U+007F..U+009F control characters
    /// - Code points defined in the Unicode specification [Unicode] to be
    ///   non-characters (for example U+0FFFF)
    /// A UTF-8 encoded sequence 0xEF 0xBB 0xBF is always interpreted as U+FEFF ("ZERO
    /// WIDTH NO-BREAK SPACE") wherever it appears in a string and MUST NOT be skipped
    /// over or stripped off by a packet receiver.
    fn verify(s: &str) -> Result<(), DataParseError> {
        if s.len() > 65535 {
            return Err(DataParseError::BadMqttUtf8String);
        }
        match s.find(|c: char| c.is_control() || c == '\u{0000}') {
            None => Ok(()),
            Some(_) => Err(DataParseError::BadMqttUtf8String),
        }
    }
}

impl MqttSerialize for MqttUtf8String {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.s.len() as u16);
        buf.put_slice(self.s.as_bytes());
    }
}

impl MqttDeserialize for MqttUtf8String {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let len = MqttTwoBytesInt::deserialize(buf)?.inner() as usize;
        if buf.remaining() < len {
            return Err(DataParseError::InsufficientBuffer {
                needed: len,
                available: buf.remaining(),
            });
        }
        let b = buf.take(len);
        let bytes = b.chunk();
        let s = std::str::from_utf8(bytes).map_err(|_| DataParseError::BadMqttUtf8String)?;
        let ret = MqttUtf8String::new(Arc::from(s));
        buf.advance(len);
        ret
    }
}

impl MqttSize for MqttUtf8String {
    fn min_size() -> usize {
        1
    }
    fn size(&self) -> usize {
        2 + self.s.len()
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttUtf8String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "\"{}\"", self.s)
    }
}

/// 1.5.5 Variable Byte Integer
#[derive(Clone)]
pub(super) struct MqttVariableBytesInt {
    i: u32,
}

impl MqttVariableBytesInt {
    pub(super) fn new(i: u32) -> Result<Self, DataParseError> {
        MqttVariableBytesInt::verify(i)?;
        Ok(MqttVariableBytesInt { i })
    }

    pub(super) fn inner(&self) -> u32 {
        self.i
    }

    fn verify(i: u32) -> Result<(), DataParseError> {
        if i > 0xfffffff {
            return Err(DataParseError::BadMqttVariableBytesInt);
        }
        Ok(())
    }
}
impl MqttSerialize for MqttVariableBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        let mut x = self.i;
        loop {
            let mut encoded_byte = (x & 0x7f) as u8; // same as x = x%128 but sans the mod part
            x >>= 7; // same as x = x / 128 but faster
            if x > 0 {
                encoded_byte |= 0x80;
            }
            buf.put_u8(encoded_byte);
            if x == 0 {
                break;
            }
        }
    }
}

impl MqttDeserialize for MqttVariableBytesInt {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let mut multiplier: u32 = 0;
        let mut value = 0;
        let mut remaining = buf.remaining();
        loop {
            if remaining == 0 {
                return Err(DataParseError::InsufficientBuffer {
                    needed: 1,
                    available: 0,
                });
            }
            remaining -= 1;
            let encoded_byte = buf.get_u8() as u32;
            value += (encoded_byte & 127) << multiplier;
            if multiplier > 21 {
                return Err(DataParseError::BadMqttVariableBytesInt);
            }
            multiplier += 7;
            if encoded_byte & 0x80 == 0 {
                break;
            }
        }
        Ok(MqttVariableBytesInt { i: value })
    }
}

impl MqttSize for MqttVariableBytesInt {
    fn min_size() -> usize {
        1
    }
    fn size(&self) -> usize {
        if self.i < 0x80 {
            1
        } else if self.i < 0x4000 {
            2
        } else if self.i < 0x200000 {
            3
        } else {
            4
        }
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttVariableBytesInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "0x{:x}", self.i)
    }
}

/// 1.5.6 Binary Data
#[derive(Clone)]
pub(super) struct MqttBinaryData {
    d: Bytes,
}

impl MqttBinaryData {
    pub(super) fn new<T: Buf>(mut buf: T) -> Result<Self, DataParseError> {
        let d = buf.copy_to_bytes(buf.remaining());
        MqttBinaryData::verify(&d)?;
        Ok(MqttBinaryData { d })
    }
    pub(super) fn inner(&self) -> &Bytes {
        &self.d
    }
    fn verify(s: &Bytes) -> Result<(), DataParseError> {
        if s.remaining() > 65535 {
            return Err(DataParseError::BadMqttBinaryData);
        }
        Ok(())
    }
}
impl MqttSerialize for MqttBinaryData {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.d.remaining() as u16);
        buf.put_slice(self.d.chunk());
    }
}

impl MqttDeserialize for MqttBinaryData {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let len = MqttTwoBytesInt::deserialize(buf)?.inner() as usize;
        if buf.remaining() < len {
            return Err(DataParseError::InsufficientBuffer {
                needed: len,
                available: buf.remaining(),
            });
        }
        let bytes = buf.copy_to_bytes(len);
        MqttBinaryData::new(bytes)
    }
}

impl MqttSize for MqttBinaryData {
    fn min_size() -> usize {
        2
    }
    fn size(&self) -> usize {
        2 + self.d.remaining()
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttBinaryData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "[")?;
        let mut bytes = self.inner().clone();
        if bytes.has_remaining() {
            let byte = bytes.get_u8();
            write!(f, "0x{:02x}", byte)?;
        }
        while bytes.has_remaining() {
            let byte = bytes.get_u8();
            write!(f, ", 0x{:02x}", byte)?;
        }
        write!(f, "]")
    }
}

#[derive(Clone)]
pub(super) struct MqttUtf8StringPair {
    pub(super) name: MqttUtf8String,
    pub(super) value: MqttUtf8String,
}

impl MqttUtf8StringPair {
    pub(super) fn new(k: Arc<str>, v: Arc<str>) -> Result<Self, DataParseError> {
        Ok(MqttUtf8StringPair {
            name: MqttUtf8String::new(k)?,
            value: MqttUtf8String::new(v)?,
        })
    }
    pub(super) fn inner(&self) -> (&Arc<str>, &Arc<str>) {
        (self.name.inner(), self.value.inner())
    }
}

impl MqttSerialize for MqttUtf8StringPair {
    fn serialize<T: BufMut>(&self, buf: &mut T) {
        self.name.serialize(buf);
        self.value.serialize(buf);
    }
}
impl MqttDeserialize for MqttUtf8StringPair {
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Ok(MqttUtf8StringPair {
            name: MqttUtf8String::deserialize(buf)?,
            value: MqttUtf8String::deserialize(buf)?,
        })
    }
}
impl MqttSize for MqttUtf8StringPair {
    fn min_size() -> usize {
        MqttUtf8String::min_size() + MqttUtf8String::min_size()
    }
    fn size(&self) -> usize {
        self.name.size() + self.value.size()
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttUtf8StringPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{{name: {:?}, value: {:?}}}", self.name, self.value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::{Bytes, BytesMut};

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_data_two_bytes_int() {
        assert_eq!(format!("{:?}", MqttTwoBytesInt(10)), "0x000a");
    }
    #[test]
    fn test_serde_data_two_bytes_int() {
        let mut buf = BytesMut::new();
        let i1 = MqttTwoBytesInt(0xff);
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0x00, 0xff]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttTwoBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.0, i1.0);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_two_bytes_int_error() {
        let mut buf = Bytes::from(&[0xff][..]);
        let i2 = MqttTwoBytesInt::deserialize(&mut buf);
        match i2 {
            Err(DataParseError::InsufficientBuffer { needed, available }) => {
                assert_eq!(needed, 2);
                assert_eq!(available, 1);
            }
            _ => panic!("Expected DataParseError::InsufficientBuffer error"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_data_four_bytes_int() {
        assert_eq!(format!("{:?}", MqttFourBytesInt(10)), "0x0000000a");
    }

    #[test]
    fn test_serde_data_four_bytes_int() {
        let mut buf = BytesMut::new();
        let i1 = MqttFourBytesInt(0xccddeeff);
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0xcc, 0xdd, 0xee, 0xff]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttFourBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.0, i1.0);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_four_bytes_int_error() {
        let mut buf = Bytes::from(&[0xff][..]);
        let i2 = MqttFourBytesInt::deserialize(&mut buf);
        match i2 {
            Err(DataParseError::InsufficientBuffer { needed, available }) => {
                assert_eq!(needed, 4);
                assert_eq!(available, 1);
            }
            _ => panic!("Expected DataParseError::InsufficientBuffer error"),
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_data_uft8_string() {
        let s = MqttUtf8String::new(Arc::from("hello")).unwrap();
        assert_eq!(format!("{:?}", s), "\"hello\"");
    }

    #[test]
    fn test_serde_data_uft8_string() {
        let mut buf = BytesMut::new();
        let s1 = MqttUtf8String::new(Arc::from("ABCD")).unwrap();
        s1.serialize(&mut buf);
        assert_eq!(&buf[..], [0x00, 0x04, 0x41, 0x42, 0x43, 0x44]);
        assert_eq!(buf.remaining(), s1.size());
        let s2 = MqttUtf8String::deserialize(&mut buf).unwrap();
        assert_eq!(s2.inner().as_ref(), "ABCD");
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_uft8_string_with_null() {
        let mut buf = BytesMut::from(&[0x00, 0x04, 0x41, 0x42, 0x00, 0x44][..]);
        match MqttUtf8String::deserialize(&mut buf) {
            Err(DataParseError::BadMqttUtf8String) => (),
            _ => panic!("Should return an error"),
        };
    }

    #[test]
    fn test_serde_data_uft8_string_with_zero_width_space() {
        let mut buf = BytesMut::from(&[0x00, 0x04, 0x41, 0xef, 0xbb, 0xbf][..]);
        let old_buf_size = buf.remaining();
        let s = MqttUtf8String::deserialize(&mut buf).unwrap();
        assert_eq!(s.inner().as_ref(), "A\u{feff}");
        assert_eq!(buf.remaining(), 0);
        assert_eq!(old_buf_size, s.size());
    }

    #[test]
    fn test_serde_data_uft8_string_with_control_character() {
        for i in 0..0x1f {
            let c = char::from_u32(i).unwrap();
            match MqttUtf8String::new(c.to_string().into_boxed_str().into()) {
                Err(DataParseError::BadMqttUtf8String) => (),
                _ => panic!("Should return an error parsing U+D800"),
            }
        }
        for i in 0x7f..0x9f {
            let c = char::from_u32(i).unwrap();
            match MqttUtf8String::new(c.to_string().into_boxed_str().into()) {
                Err(DataParseError::BadMqttUtf8String) => (),
                _ => panic!("Should return an error parsing U+D800"),
            }
        }
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_data_data_variable_byte_int_string() {
        let d = MqttVariableBytesInt::new(0x11).unwrap();
        assert_eq!(format!("{:?}", d), "0x11");
    }

    #[test]
    fn test_serde_data_variable_byte_int_11() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0x11).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0x11]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_variable_byte_int_80() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(128).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0x80, 0x1]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_variable_byte_int_3fff() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0x3fff).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0xff, 0x7f]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_variable_byte_int_4000() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0x4000).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0x80, 0x80, 0x01]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_variable_byte_int_1fffff() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0x1fffff).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0xff, 0xff, 0x7f]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_variable_byte_int_200000() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0x200000).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0x80, 0x80, 0x80, 0x01]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }

    #[test]
    fn test_serde_data_variable_byte_int_fffffff() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0xfffffff).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0xff, 0xff, 0xff, 0x7f]);
        assert_eq!(buf.remaining(), i1.size());
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
        assert_eq!(buf.remaining(), 0);
    }
    #[test]
    fn test_serde_data_variable_byte_int_invalid() {
        let mut buf = BytesMut::new();
        let i1 = MqttVariableBytesInt::new(0xfffffff).unwrap();
        i1.serialize(&mut buf);
        assert_eq!(&buf[..], [0xff, 0xff, 0xff, 0x7f]);
        let i2 = MqttVariableBytesInt::deserialize(&mut buf).unwrap();
        assert_eq!(i2.i, i1.i);
    }

    #[test]
    fn test_serde_data_variable_byte_int_invalid2() {
        let mut buf = BytesMut::from(&[0xff, 0xff, 0xff, 0x80, 0x1][..]);
        let i2 = MqttVariableBytesInt::deserialize(&mut buf);
        match i2 {
            Err(DataParseError::BadMqttVariableBytesInt) => (),
            _ => panic!("Expected DataParseError::BadMqttVariableBytesInt"),
        };
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_data_binary_data_string() {
        let d = MqttBinaryData::new(Bytes::from(&[0x1, 0x2, 0x3, 0x4][..])).unwrap();
        assert_eq!(format!("{:?}", d), "[0x01, 0x02, 0x03, 0x04]");
    }

    #[test]
    fn test_serde_string_max() {
        let mut b = BytesMut::with_capacity(0x10000);
        b.put_bytes(0x41, 0xffff);
        let bytes = b.chunk();
        let s = Arc::from(std::str::from_utf8(bytes).unwrap());
        MqttUtf8String::new(s).unwrap();
        b.put_u8(0x41);
        let bytes = b.chunk();
        let s = Arc::from(std::str::from_utf8(bytes).unwrap());
        let d = MqttUtf8String::new(s);
        match d {
            Err(DataParseError::BadMqttUtf8String) => (),
            _ => panic!("expected a DataParseError::BadMqttUtf8String error"),
        }
    }

    #[test]
    fn test_serde_data_max() {
        let mut b = BytesMut::with_capacity(0x10000);
        b.put_bytes(0x41, 0xffff);
        MqttBinaryData::new(b.clone()).unwrap();
        b.put_u8(0x41);
        let d = MqttBinaryData::new(b.clone());
        match d {
            Err(DataParseError::BadMqttBinaryData) => (),
            _ => panic!("expected a DataParseError::BadMqttBinaryData error"),
        }
    }

    #[test]
    fn test_serde_data() {
        let d = MqttBinaryData::new(Bytes::from(&[0x1, 0x2, 0x3, 0x4][..])).unwrap();
        let mut b = BytesMut::new();
        d.serialize(&mut b);
        assert_eq!(b, &[0x0, 0x4, 0x1, 0x2, 0x3, 0x4][..]);
        let d2 = MqttBinaryData::deserialize(&mut b).unwrap();
        assert_eq!(d.inner(), d2.inner());
        assert_eq!(b.remaining(), 0);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_string_pair() {
        let d = MqttUtf8StringPair {
            name: MqttUtf8String::new(Arc::from("Hello")).unwrap(),
            value: MqttUtf8String::new(Arc::from("world")).unwrap(),
        };
        assert_eq!(format!("{:?}", d), "{name: \"Hello\", value: \"world\"}");
    }

    #[test]
    fn test_serde_string_pair() {
        let d1 = MqttUtf8StringPair {
            name: MqttUtf8String::new(Arc::from("Hello")).unwrap(),
            value: MqttUtf8String::new(Arc::from("world")).unwrap(),
        };
        let mut b = BytesMut::new();
        d1.serialize(&mut b);
        assert_eq!(
            &b[..],
            &[0x0, 0x5, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x0, 0x5, 0x77, 0x6f, 0x72, 0x6c, 0x64][..]
        );
        assert_eq!(b.remaining(), d1.size());
        let d2 = MqttUtf8StringPair::deserialize(&mut b).unwrap();
        assert_eq!(d1.name.inner(), d2.name.inner());
        assert_eq!(d1.value.inner(), d2.value.inner());
        assert_eq!(b.remaining(), 0);
    }

    #[test]
    fn test_incomplete_string() {
        let mut b = Bytes::from(&[][..]);
        let d = MqttUtf8String::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 2,
                available: 0
            }
        );
        let mut b = Bytes::from(&[0x00][..]);
        let d = MqttUtf8String::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 2,
                available: 1
            }
        );
        let mut b = Bytes::from(&[0x00, 0x05][..]);
        let d = MqttUtf8String::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 5,
                available: 0
            }
        );
        let mut b = Bytes::from(&[0x00, 0x05, 0x04][..]);
        let d = MqttUtf8String::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 5,
                available: 1
            }
        );
    }

    #[test]
    fn test_incomplete_data() {
        let mut b = Bytes::from(&[][..]);
        let d = MqttBinaryData::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 2,
                available: 0
            }
        );
        let mut b = Bytes::from(&[0x00][..]);
        let d = MqttBinaryData::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 2,
                available: 1
            }
        );
        let mut b = Bytes::from(&[0x00, 0x05][..]);
        let d = MqttBinaryData::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 5,
                available: 0
            }
        );
        let mut b = Bytes::from(&[0x00, 0x05, 0x04][..]);
        let d = MqttBinaryData::deserialize(&mut b);
        assert_eq!(
            d.err().unwrap(),
            DataParseError::InsufficientBuffer {
                needed: 5,
                available: 1
            }
        );
    }
}
