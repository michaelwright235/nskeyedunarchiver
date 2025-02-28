//! These Decodable impls only exit to make the code simpler by calling `get_from_object`
//! with a unified trait.
//! Actually it's logically incorrect because we implement Decodable only for archive objects.

use super::{Decodable, NSArray, ObjectType};
use crate::{DeError, Object, UniqueId, ValueRef};

impl Decodable for bool {
    fn get_from_object(obj: &Object, key: &str, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        obj.decode_bool(key)
    }

    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(_value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Err(DeError::Message("Don't use it this way!".into()))
    }
}

impl Decodable for Vec<u8> {
    fn get_from_object(obj: &Object, key: &str, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        obj.decode_data(key).map(|v| v.to_vec())
    }

    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(_value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Err(DeError::Message("Don't use it this way!".into()))
    }
}

impl<T> Decodable for Vec<T>
where
    T: Decodable,
{
    fn get_from_object(obj: &Object, key: &str, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let arr = NSArray::get_from_object(obj, key, types)?;
        arr.try_into_objects::<T>()
    }

    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(_value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Err(DeError::Message("Don't use it this way!".into()))
    }
}

impl Decodable for ValueRef {
    fn get_from_object(obj: &Object, key: &str, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        obj.decode_object(key)
    }

    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(_value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Err(DeError::Message("Don't use it this way!".into()))
    }
}

impl Decodable for UniqueId {
    fn get_from_object(obj: &Object, key: &str, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        obj.decode_object(key).map(|v| v.unique_id)
    }

    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(_value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Err(DeError::Message("Don't use it this way!".into()))
    }
}

impl<T> Decodable for Option<T>
where
    T: Decodable,
{
    fn get_from_object(obj: &Object, key: &str, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Ok(T::get_from_object(obj, key, types).ok())
    }

    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(_value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Err(DeError::Message("Don't use it this way!".into()))
    }
}
