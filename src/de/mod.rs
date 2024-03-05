mod impls;
pub use impls::*;

use crate::{DeError, ValueRef};
use std::any::{Any, TypeId};

/// A trait that can be implemented for a structure to be decodable.
pub trait Decodable: Sized {
    /// This method should return `true` if a structure that implements this method
    /// is logically represents the object.
    ///
    /// Usually you only need to check the first value from the given `classes` string slice
    /// that represents the main class. However you can also check the other ones which
    /// are the parents of this class.
    fn is_type_of(classes: &[String]) -> bool;

    /// The main decoding method of your structure
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError>;

    #[doc(hidden)]
    /// This is an internal method that usually shouldn't be overwritten.
    fn decode_as_any(
        value: ValueRef,
        types: &[ObjectType],
    ) -> Result<Box<dyn std::any::Any>, DeError>
    where
        Self: 'static,
    {
        Ok(Box::new(Self::decode(value, types)?) as Box<dyn std::any::Any>)
    }

    #[doc(hidden)]
    /// This is an internal method that usually shouldn't be overwritten.
    fn as_object_type() -> ObjectType
    where
        Self: 'static,
    {
        ObjectType::new(TypeId::of::<Self>(), Self::is_type_of, Self::decode_as_any)
    }
}

type IsTypeOfFn = fn(classes: &[String]) -> bool;
type DecodeAsAnyFn = fn(obj: ValueRef, types: &[ObjectType]) -> Result<Box<dyn Any>, DeError>;

#[doc(hidden)]
pub struct ObjectType(TypeId, IsTypeOfFn, DecodeAsAnyFn);
impl ObjectType {
    pub fn new(t: TypeId, c: IsTypeOfFn, d: DecodeAsAnyFn) -> Self {
        Self(t, c, d)
    }
    pub fn type_id(&self) -> TypeId {
        self.0
    }
    pub fn is_type_of(&self, classes: &[String]) -> bool {
        self.1(classes)
    }
    pub fn decode(&self, obj: ValueRef, types: &[ObjectType]) -> Result<Box<dyn Any>, DeError> {
        self.2(obj, types)
    }
}

#[macro_export]
macro_rules! object_types {
    ($($name:ident),*) => {
        Vec::from([
            $crate::de::NSArray::as_object_type(),
            $crate::de::NSSet::as_object_type(),
            $crate::de::NSDictionary::as_object_type(),
            $crate::de::NSData::as_object_type(),
            $(
                $name::as_object_type()
            ),*
        ])
    };
}

#[macro_export]
macro_rules! object_types_empty {
    ($($name:ident),*) => {
        Vec::from([
            $(
                $name::as_object_type()
            ),*
        ])
    };
}

#[macro_export]
macro_rules! as_object {
    ($obj_ref:ident) => {{
        $obj_ref.as_object().ok_or($crate::DeError::ExpectedObject)
    }};
}

pub fn value_ref_to_any(
    value_ref: ValueRef,
    types: &[ObjectType],
) -> Result<Box<dyn Any>, DeError> {
    let Some(object) = value_ref.as_object() else {
        return Err(DeError::ExpectedObject);
    };
    let classes = object.classes();
    let mut result = None;
    for typ in types {
        if typ.is_type_of(classes) {
            result = Some(typ.decode(value_ref.clone(), types));
        }
    }
    match result {
        Some(val) => val,
        None => Err(DeError::Message(format!(
            "Undecodable object: {}",
            classes[0]
        ))),
    }
}
