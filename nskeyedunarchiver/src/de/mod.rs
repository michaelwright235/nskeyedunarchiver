mod impls;
use downcast_rs::{impl_downcast, Downcast};
pub use impls::*;

use crate::{DeError, ObjectValue, ValueRef};
use std::any::TypeId;

#[cfg(not(feature = "debug_decodable"))]
/// A trait that can be implemented for a structure to be decodable.
pub trait Decodable: Downcast {
    /// This method should return `true` if a structure that implements this method
    /// is logically represents the object.
    ///
    /// Usually you only need to check the first value from the given `classes` string slice
    /// that represents the main class. However you can also check the other ones which
    /// are the parents of this class.
    fn is_type_of(classes: &[String]) -> bool
    where
        Self: Sized;

    fn class(&self) -> &str;

    /// The main decoding method of your structure
    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized;

    #[doc(hidden)]
    /// This is an internal method that usually shouldn't be overwritten.
    fn decode_as_any(
        value: &ObjectValue,
        types: &[ObjectType],
    ) -> Result<Box<dyn Decodable>, DeError>
    where
        Self: Sized + 'static,
    {
        Ok(Box::new(Self::decode(value, types)?) as Box<dyn Decodable>)
    }

    #[doc(hidden)]
    /// This is an internal method that usually shouldn't be overwritten.
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        Some(ObjectType::new::<Self>())
    }
}

#[cfg(feature = "debug_decodable")]
/// A trait that can be implemented for a structure to be decodable.
pub trait Decodable: Downcast + std::fmt::Debug {
    /// This method should return `true` if a structure that implements this method
    /// is logically represents the object.
    ///
    /// Usually you only need to check the first value from the given `classes` string slice
    /// that represents the main class. However you can also check the other ones which
    /// are the parents of this class.
    fn is_type_of(classes: &[String]) -> bool
    where
        Self: Sized;

    fn class(&self) -> &str;

    /// The main decoding method of your structure
    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized;

    #[doc(hidden)]
    /// This is an internal method that usually shouldn't be overwritten.
    fn decode_as_any(
        value: &ObjectValue,
        types: &[ObjectType],
    ) -> Result<Box<dyn Decodable>, DeError>
    where
        Self: Sized + 'static,
    {
        Ok(Box::new(Self::decode(value, types)?) as Box<dyn Decodable>)
    }

    #[doc(hidden)]
    /// This is an internal method that usually shouldn't be overwritten.
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        Some(ObjectType::new::<Self>())
    }
}

impl_downcast!(Decodable);

#[cfg(not(feature = "debug_decodable"))]
impl std::fmt::Debug for dyn Decodable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Decodable: {} {{ ... }}", self.class()))
    }
}

type IsTypeOfFnType = fn(classes: &[String]) -> bool;
type DecodeAsAnyFnType =
    fn(obj: &ObjectValue, types: &[ObjectType]) -> Result<Box<dyn Decodable>, DeError>;

#[doc(hidden)]
#[derive(PartialEq, Clone, Debug)]
pub struct ObjectType {
    type_id: TypeId,
    is_type_of_fn: IsTypeOfFnType,
    decode_as_any_fn: DecodeAsAnyFnType,
}

impl ObjectType {
    pub fn new<T: Decodable>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            is_type_of_fn: T::is_type_of,
            decode_as_any_fn: T::decode_as_any,
        }
    }
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
    pub fn is_type_of(&self, classes: &[String]) -> bool {
        (self.is_type_of_fn)(classes)
    }
    pub fn decode_as_any(
        &self,
        obj: &ObjectValue,
        types: &[ObjectType],
    ) -> Result<Box<dyn Decodable>, DeError> {
        (self.decode_as_any_fn)(obj, types)
    }
}

#[macro_export]
macro_rules! object_types {
    ($($name:ident),*) => {{
        use $crate::de::Decodable;
        Vec::from([
            <$crate::de::NSArray as Decodable>::as_object_type().unwrap(),
            <$crate::de::NSSet as Decodable>::as_object_type().unwrap(),
            <$crate::de::NSDictionary as Decodable>::as_object_type().unwrap(),
            <$crate::de::NSData as Decodable>::as_object_type().unwrap(),
            $(
                <$name as Decodable>::as_object_type().unwrap()
            ),*
        ])
    }};
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
        let $crate::ObjectValue::Ref(value) = $obj_ref else {
            return Err($crate::DeError::ExpectedObject);
        };
        value.as_object().ok_or($crate::DeError::ExpectedObject)
    }};
}

pub fn value_ref_to_any(
    value_ref: ValueRef,
    types: &[ObjectType],
) -> Result<Box<dyn Decodable>, DeError> {
    let Some(object) = value_ref.as_object() else {
        return Err(DeError::ExpectedObject);
    };
    let classes = object.classes();
    let mut result = None;
    for typ in types {
        if typ.is_type_of(classes) {
            result = Some(typ.decode_as_any(&value_ref.clone().into(), types));
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
