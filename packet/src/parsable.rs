use crate::error::DataParseError;
use bytes::{Buf, BufMut};

pub(super) trait UncheckedParsable {
    fn unchecked_serialize<T: BufMut>(&self, buf: &mut T);
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Self;
}

pub(super) trait Parsable {
    fn serialize<T: BufMut>(&self, buf: &mut T) -> Result<(), DataParseError>;
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError>
    where
        Self: Sized;
    fn size(&self) -> usize;
}
