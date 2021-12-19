// Data representation for MQTT v5.0 as per section 1.5

use crate::parsable::{DataParseError, Parsable, UncheckedParsable};
use bytes::{Buf, BufMut, Bytes};
#[cfg(feature = "debug")]
use std::fmt;

pub struct MqttOneBytesInt(u8);
impl MqttOneBytesInt {
    pub fn new(i: u8) -> Self {
        MqttOneBytesInt(i)
    }
    pub fn inner(&self) -> u8 {
        self.0
    }
}
impl UncheckedParsable for MqttOneBytesInt {
    fn unchecked_serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(self.0)
    }
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Self {
        MqttOneBytesInt(buf.get_u8())
    }
}

impl Parsable for MqttOneBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.unchecked_serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let size = buf.remaining();
        if size < 1 {
            Err(DataParseError::InsufficientBuffer {
                needed: 1,
                available: size,
            })
        } else {
            Ok(Self::unchecked_deserialize(buf))
        }
    }
    fn size(&self) -> usize {
        1
    }
}

/// 1.5.2 Two Byte Integer
pub struct MqttTwoBytesInt(u16);

impl MqttTwoBytesInt {
    pub fn new(i: u16) -> Self {
        MqttTwoBytesInt(i)
    }
    pub fn inner(&self) -> u16 {
        self.0
    }
}

impl UncheckedParsable for MqttTwoBytesInt {
    fn unchecked_serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.0)
    }
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Self {
        MqttTwoBytesInt(buf.get_u16())
    }
}

impl Parsable for MqttTwoBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.unchecked_serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let size = buf.remaining();
        if size < 2 {
            Err(DataParseError::InsufficientBuffer {
                needed: 2,
                available: size,
            })
        } else {
            Ok(Self::unchecked_deserialize(buf))
        }
    }
    fn size(&self) -> usize {
        2
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttTwoBytesInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "0x{:04x}", self.0)
    }
}

/// 1.5.3 Four Byte Integer
pub struct MqttFourBytesInt(u32);

impl MqttFourBytesInt {
    pub fn new(i: u32) -> Self {
        MqttFourBytesInt(i)
    }
    pub fn inner(&self) -> u32 {
        self.0
    }
}

impl UncheckedParsable for MqttFourBytesInt {
    fn unchecked_serialize<T: BufMut>(&self, buf: &mut T) {
        buf.put_u32(self.0)
    }
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Self {
        MqttFourBytesInt(buf.get_u32())
    }
}

impl Parsable for MqttFourBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.unchecked_serialize(buf);
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let size = buf.remaining();
        if size < 4 {
            Err(DataParseError::InsufficientBuffer {
                needed: 4,
                available: size,
            })
        } else {
            Ok(Self::unchecked_deserialize(buf))
        }
    }
    fn size(&self) -> usize {
        4
    }
}

#[cfg(feature = "debug")]
impl fmt::Debug for MqttFourBytesInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "0x{:08x}", self.0)
    }
}

/// 1.5.4 UTF-8 Encoded String
pub struct MqttUtf8String {
    s: String,
}

impl MqttUtf8String {
    pub fn new(s: String) -> Result<Self, DataParseError> {
        MqttUtf8String::verify(&s)?;
        Ok(MqttUtf8String { s })
    }

    pub fn unwrap(self) -> String {
        self.s
    }

    pub fn inner(&self) -> &str {
        &self.s
    }

    pub fn inner_mut(&mut self) -> &mut String {
        &mut self.s
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

impl Parsable for MqttUtf8String {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        buf.put_u16(self.s.len() as u16);
        buf.put_slice(self.s.as_bytes());
        Ok(())
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let len = MqttTwoBytesInt::deserialize(buf)?.inner() as usize;
        if buf.remaining() < len {
            return Err(DataParseError::InsufficientBuffer {
                needed: len,
                available: buf.remaining(),
            });
        }
        //TODO copy is happening here optimize later
        let bytes = Vec::from(buf.take(len).chunk());
        let s = String::from_utf8(bytes).map_err(|_| DataParseError::BadMqttUtf8String)?;
        buf.advance(len);
        MqttUtf8String::new(s)
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
pub struct MqttVariableBytesInt {
    i: u32,
}

impl MqttVariableBytesInt {
    pub fn new(i: u32) -> Result<Self, DataParseError> {
        MqttVariableBytesInt::verify(i)?;
        Ok(MqttVariableBytesInt { i })
    }

    pub fn inner(&self) -> u32 {
        self.i
    }

    fn verify(i: u32) -> Result<(), DataParseError> {
        if i > 0xfffffff {
            return Err(DataParseError::BadMqttVariableBytesInt);
        }
        Ok(())
    }
}

impl Parsable for MqttVariableBytesInt {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
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
        Ok(())
    }
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
pub struct MqttBinaryData {
    d: Bytes,
}

impl MqttBinaryData {
    pub fn new<T: Buf>(mut buf: T) -> Result<Self, DataParseError> {
        let d = buf.copy_to_bytes(buf.remaining());
        MqttBinaryData::verify(&d)?;
        Ok(MqttBinaryData { d })
    }

    pub fn unwrap(self) -> Bytes {
        self.d
    }

    pub fn inner(&self) -> &Bytes {
        &self.d
    }

    pub fn inner_mut(&mut self) -> &mut Bytes {
        &mut self.d
    }

    fn verify(s: &Bytes) -> Result<(), DataParseError> {
        if s.remaining() > 65535 {
            return Err(DataParseError::BadMqttBinaryData);
        }
        Ok(())
    }
}

impl Parsable for MqttBinaryData {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        buf.put_u16(self.d.remaining() as u16);
        buf.put_slice(self.d.chunk());
        Ok(())
    }
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

pub struct MqttUtf8StringPair {
    pub name: MqttUtf8String,
    pub value: MqttUtf8String,
}

impl Parsable for MqttUtf8StringPair {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError> {
        self.name.serialize(buf)?;
        self.value.serialize(buf)
    }
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        Ok(MqttUtf8StringPair {
            name: MqttUtf8String::deserialize(buf)?,
            value: MqttUtf8String::deserialize(buf)?,
        })
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        let s = MqttUtf8String::new("hello".to_string()).unwrap();
        assert_eq!(format!("{:?}", s), "\"hello\"");
    }

    #[test]
    fn test_serde_data_uft8_string() {
        let mut buf = BytesMut::new();
        let s1 = MqttUtf8String::new("ABCD".to_string()).unwrap();
        s1.serialize(&mut buf).unwrap();
        assert_eq!(&buf[..], [0x00, 0x04, 0x41, 0x42, 0x43, 0x44]);
        assert_eq!(buf.remaining(), s1.size());
        let s2 = MqttUtf8String::deserialize(&mut buf).unwrap();
        assert_eq!(s2.inner(), "ABCD");
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
        assert_eq!(s.inner(), "A\u{feff}");
        assert_eq!(buf.remaining(), 0);
        assert_eq!(old_buf_size, s.size());
    }

    #[test]
    fn test_serde_data_uft8_string_with_control_character() {
        for i in 0..0x1f {
            let c = char::from_u32(i).unwrap();
            match MqttUtf8String::new(c.to_string()) {
                Err(DataParseError::BadMqttUtf8String) => (),
                _ => panic!("Should return an error parsing U+D800"),
            }
        }
        for i in 0x7f..0x9f {
            let c = char::from_u32(i).unwrap();
            match MqttUtf8String::new(c.to_string()) {
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        i1.serialize(&mut buf).unwrap();
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
        let bytes = Vec::from(b.chunk());
        let s = String::from_utf8(bytes).unwrap();
        MqttUtf8String::new(s).unwrap();
        b.put_u8(0x41);
        let bytes = Vec::from(b.chunk());
        let s = String::from_utf8(bytes).unwrap();
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
        d.serialize(&mut b).unwrap();
        assert_eq!(b, &[0x0, 0x4, 0x1, 0x2, 0x3, 0x4][..]);
        let d2 = MqttBinaryData::deserialize(&mut b).unwrap();
        assert_eq!(d.inner(), d2.inner());
        assert_eq!(b.remaining(), 0);
    }

    #[test]
    #[cfg(feature = "debug")]
    fn test_format_string_pair() {
        let d = MqttUtf8StringPair {
            name: MqttUtf8String::new("Hello".to_string()).unwrap(),
            value: MqttUtf8String::new("world".to_string()).unwrap(),
        };
        assert_eq!(format!("{:?}", d), "{name: \"Hello\", value: \"world\"}");
    }

    #[test]
    fn test_serde_string_pair() {
        let d1 = MqttUtf8StringPair {
            name: MqttUtf8String::new("Hello".to_string()).unwrap(),
            value: MqttUtf8String::new("world".to_string()).unwrap(),
        };
        let mut b = BytesMut::new();
        d1.serialize(&mut b).unwrap();
        assert_eq!(
            b,
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
