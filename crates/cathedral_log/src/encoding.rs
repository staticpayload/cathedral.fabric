//! Canonical encoding

use cathedral_core::error::CoreResult;

pub trait CanonicalEncode {
    fn encode(&self) -> Vec<u8>;
}

pub trait CanonicalDecode {
    fn decode(data: &[u8]) -> CoreResult<Self> where Self: Sized;
}
