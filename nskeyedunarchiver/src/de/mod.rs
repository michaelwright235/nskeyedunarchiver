mod impls;
use crate::{DeError, ObjectValue};

/// A trait that can be implemented for a structure to be decodable.
pub trait Decodable {
    /// The main decoding method of your structure
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized;
}
