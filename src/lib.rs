//! # heapless-bytes
//!
//! Newtype around heapless byte Vec with efficient serde.

#![no_std]

use core::{
    cmp::Ordering,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub use heapless::consts;
pub use heapless::ArrayLength;
use heapless::Vec;

use serde::{
    de::{Deserialize, Deserializer, Error as _, SeqAccess, Visitor},
    ser::{Serialize, Serializer},
};

#[derive(Clone, Default, Eq)]
pub struct Bytes<N: ArrayLength<u8>> {
    bytes: Vec<u8, N>,
}

impl<N: ArrayLength<u8>> Bytes<N> {
    /// Construct a new, empty `Bytes<N>`.
    pub fn new() -> Self {
        Bytes::from(Vec::new())
    }

    // /// Construct a new, empty `Bytes<N>` with the specified capacity.
    // pub fn with_capacity(cap: usize) -> Self {
    //     Bytes<N>::from(Vec::with_capacity(cap))
    // }

    /// Wrap existing bytes in a `Bytes<N>`.
    pub fn from<T: Into<Vec<u8, N>>>(bytes: T) -> Self {
        Bytes {
            bytes: bytes.into(),
        }
    }

    /// Unwrap the vector of byte underlying this `Bytes<N>`.
    pub fn into_vec(self) -> Vec<u8, N> {
        self.bytes
    }

    // #[doc(hidden)]
    // pub fn into_iter(self) -> <Vec<u8, N> as IntoIterator>::IntoIter {
    //     self.bytes.into_iter()
    // }

    pub fn try_from_slice(slice: &[u8]) -> core::result::Result<Self, ()> {
        let mut bytes = Vec::<u8, N>::new();
        bytes.extend_from_slice(slice)?;
        Ok(Self::from(bytes))
    }

    // cf. https://internals.rust-lang.org/t/add-vec-insert-slice-at-to-insert-the-content-of-a-slice-at-an-arbitrary-index/11008/3
    pub fn insert_slice_at(&mut self, slice: &[u8], at: usize) -> core::result::Result<(), ()> {
        let l = slice.len();
        let before = self.len();

        // make space
        self.bytes.resize_default(before + l)?;

        // move back existing
        let raw: &mut [u8] = &mut self.bytes;
        raw.copy_within(at..before, at + l);

        // insert slice
        raw[at..][..l].copy_from_slice(slice);

        Ok(())
    }

    // pub fn deref_mut(&mut self) -> &mut [u8] {
    //     self.bytes.deref_mut()
    // }

    #[cfg(feature = "cbor")]
    pub fn from_serialized<T>(t: &T) -> Self
    where
        T: Serialize,
    {
        let mut vec = Vec::<u8, N>::new();
        vec.resize_default(N::to_usize()).unwrap();
        let buffer = vec.deref_mut();

        let writer = serde_cbor::ser::SliceWrite::new(buffer);
        let mut ser = serde_cbor::Serializer::new(writer)
            .packed_format()
            // .pack_starting_with(1)
            // .pack_to_depth(1)
        ;
        t.serialize(&mut ser).unwrap();
        let writer = ser.into_inner();
        let size = writer.bytes_written();
        vec.resize_default(size).unwrap();
        Self::from(vec)
    }
}

impl<N: ArrayLength<u8>> Debug for Bytes<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: There has to be a better way :'-)

        use core::ascii::escape_default;
        f.write_str("b'")?;
        for byte in &self.bytes {
            for ch in escape_default(*byte) {
                // Debug::fmt(unsafe { core::str::from_utf8_unchecked(&[ch]) }, f)?;
                f.write_str(unsafe { core::str::from_utf8_unchecked(&[ch]) })?;
                // f.write(&ch);
            }
        }
        f.write_str("'")?;
        Ok(())
    }
}

impl<N: ArrayLength<u8>> AsRef<[u8]> for Bytes<N> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<N: ArrayLength<u8>> AsMut<[u8]> for Bytes<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

impl<N: ArrayLength<u8>> Deref for Bytes<N> {
    type Target = Vec<u8, N>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl<N: ArrayLength<u8>> DerefMut for Bytes<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

// impl Borrow<Bytes> for Bytes<N> {
//     fn borrow(&self) -> &Bytes {
//         Bytes::new(&self.bytes)
//     }
// }

// impl BorrowMut<Bytes> for Bytes<N> {
//     fn borrow_mut(&mut self) -> &mut Bytes {
//         unsafe { &mut *(&mut self.bytes as &mut [u8] as *mut [u8] as *mut Bytes) }
//     }
// }

impl<N: ArrayLength<u8>, Rhs> PartialEq<Rhs> for Bytes<N>
where
    Rhs: ?Sized + AsRef<[u8]>,
{
    fn eq(&self, other: &Rhs) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<N: ArrayLength<u8>, Rhs> PartialOrd<Rhs> for Bytes<N>
where
    Rhs: ?Sized + AsRef<[u8]>,
{
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<N: ArrayLength<u8>> Hash for Bytes<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<N: ArrayLength<u8>> IntoIterator for Bytes<N> {
    type Item = u8;
    type IntoIter = <Vec<u8, N> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes.into_iter()
    }
}

impl<'a, N: ArrayLength<u8>> IntoIterator for &'a Bytes<N> {
    type Item = &'a u8;
    type IntoIter = <&'a [u8] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes.iter()
    }
}

impl<'a, N: ArrayLength<u8>> IntoIterator for &'a mut Bytes<N> {
    type Item = &'a mut u8;
    type IntoIter = <&'a mut [u8] as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes.iter_mut()
    }
}

impl<N> Serialize for Bytes<N>
where
    N: ArrayLength<u8>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self)
    }
}

// TODO: can we delegate to Vec<u8, N> deserialization instead of reimplementing?
impl<'de, N> Deserialize<'de> for Bytes<N>
where
    N: ArrayLength<u8>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor<'de, N>(PhantomData<(&'de (), N)>);

        impl<'de, N> Visitor<'de> for ValueVisitor<'de, N>
        where
            N: ArrayLength<u8>,
        {
            // type Value = Vec<T, N>;
            type Value = Bytes<N>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values: Vec<u8, N> = Vec::new();
                // hprintln!("made a values: {:?} of capacity {:?}",
                //           &values, N::to_usize()).ok();

                while let Some(value) = seq.next_element()? {
                    if values.push(value).is_err() {
                        // hprintln!("error! {}", values.capacity() + 1).ok();
                        // hprintln!("pushing value {:?} errored", &value).ok();
                        return Err(A::Error::invalid_length(values.capacity() + 1, &self))?;
                    }
                }

                Ok(Bytes::from(values))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() > N::to_usize() {
                    // hprintln!("error! own size: {}, data size: {}", N::to_usize(), v.len()).ok();
                    // return Err(E::invalid_length(values.capacity() + 1, &self))?;
                    return Err(E::invalid_length(v.len(), &self))?;
                }
                let mut buf: Vec<u8, N> = Vec::new();
                // avoid unwrapping even though redundant
                match buf.extend_from_slice(v) {
                    Ok(()) => {}
                    Err(()) => {
                        // hprintln!("error! own size: {}, data size: {}", N::to_usize(), v.len()).ok();
                        // return Err(E::invalid_length(values.capacity() + 1, &self))?;
                        return Err(E::invalid_length(v.len(), &self))?;
                    }
                }
                Ok(Bytes::<N>::from(buf))
            }
        }
        deserializer.deserialize_seq(ValueVisitor(PhantomData))
    }
}

#[cfg(test)]
#[cfg(feature = "cbor")]
mod tests {
    use super::*;
    use heapless::consts;

    #[test]
    fn test_client_data_hash() {
        let mut minimal = [
            0x50u8, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x41, 0x42, 0x43,
            0x44, 0x45, 0x46,
        ];

        let client_data_hash: Bytes<consts::U32> =
            serde_cbor::de::from_mut_slice(&mut minimal).unwrap();

        assert_eq!(client_data_hash, b"1234567890ABCDEF");
    }
}