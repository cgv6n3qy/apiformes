use crate::error::DataParseError;
use bytes::{Buf, BufMut};

/// This trait implements helper functions for deserializing data structures that have fixed size
/// without performing any bound checks.
pub(super) trait MqttUncheckedDeserialize {
    /// Returns the size of `Self`
    fn fixed_size() -> usize;
    /// Deserialize a `Self` type from `buf` without checking for sufficient capacity in `buf`.
    ///
    /// # Return value
    /// This function returns appropriate `DataParseError` if `buf` contains bytes that represent
    /// malformed packet. This method does not guarantee checking for bound errors.
    ///
    /// # Panic
    /// This function will not directly perform any bound checks, and will panic when there is no
    /// sufficient bytes in `buf`.
    fn unchecked_deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError>
    where
        Self: Sized;
}

pub(super) trait MqttSize {
    /// Returns the number of bytes required to serialize/deserialize
    /// `self`
    fn size(&self) -> usize;
    /// Returns the number of bytes required to serialize/deserialize
    /// the smallest element in `Self`
    fn min_size() -> usize;
}

impl<T> MqttSize for T
where
    T: MqttUncheckedDeserialize,
{
    #[inline(always)]
    fn size(&self) -> usize {
        T::fixed_size()
    }
    #[inline(always)]
    fn min_size() -> usize {
        T::fixed_size()
    }
}

pub(super) trait MqttDeserialize {
    /// Deserialize a `Self` from `buf`
    ///
    /// # Error handling
    /// If there is not sufficient data in `buf` to deserialize `Self`,
    /// `DataParseError::InsufficientBuffer` will be returned;
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError>
    where
        Self: Sized;
}

impl<S> MqttDeserialize for S
where
    S: MqttUncheckedDeserialize,
{
    #[inline(always)]
    fn deserialize<T: Buf>(buf: &mut T) -> Result<Self, DataParseError> {
        let available = buf.remaining();
        let needed = Self::fixed_size();
        if available < needed {
            Err(DataParseError::InsufficientBuffer { needed, available })
        } else {
            Self::unchecked_deserialize(buf)
        }
    }
}

pub(super) trait MqttSerialize {
    /// Serialize `Self` into `buf`
    fn serialize<T: BufMut>(&self, buf: &mut T);
}
