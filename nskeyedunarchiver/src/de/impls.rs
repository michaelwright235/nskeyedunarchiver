use super::{Decodable, ObjectType};
use crate::{as_object, DeError, Integer, ObjectValue, UniqueId, ValueRef};
use std::collections::HashMap;

impl Decodable for String {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSString" || classes[0] == "NSMutableString"
    }
    fn class(&self) -> &str {
        "NSString"
    }
    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError> {
        // A string can be encoded as a plain String type
        if let ObjectValue::String(s) = value {
            return Ok(s.to_string());
        }

        // ... or as an Object with `NS.bytes` data or `NS.string` string (NIB Archives)
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedString);
        };

        if let Some(s) = value.as_string() {
            return Ok(s.into());
        }

        if !value.is_object() {
            return Err(DeError::ExpectedString);
        }

        let obj = value.as_object().unwrap();
        if obj.class() != "NSString" && obj.class() != "NSMutableString" {
            return Err(DeError::Message(format!(
                "Incorrect value type of '{0}' for object '{1}'. Expected '{2}'",
                obj.class(),
                "NSString",
                "NSString or NSMutableString",
            )));
        }

        if !obj.contains_key("NS.bytes") && !obj.contains_key("NS.string") {
            return Err(DeError::ExpectedString);
        }
        let s = if let Some(ObjectValue::Data(data)) = obj.as_map().get("NS.bytes") {
            let parsed = String::from_utf8(data.to_vec());
            if let Err(e) = parsed {
                return Err(DeError::Message(format!(
                    "Unable to parse a UTF-8 string: {e}"
                )));
            }
            parsed.unwrap()
        } else if let Some(ObjectValue::String(data)) = obj.as_map().get("NS.string") {
            data.clone()
        } else {
            return Err(DeError::ExpectedString);
        };
        Ok(s)
    }
}

impl Decodable for bool {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        if let ObjectValue::Boolean(value) = value {
            return Ok(*value);
        }
        if let ObjectValue::Ref(value) = value {
            if let Some(v) = value.as_boolean() {
                return Ok(v);
            }
        }
        Err(DeError::ExpectedBoolean)
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for Vec<u8> {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        if let ObjectValue::Data(value) = value {
            return Ok(value.to_vec());
        }
        if let ObjectValue::Ref(value) = value {
            if let Some(v) = value.as_data() {
                return Ok(v.to_vec());
            }
            // Decoding NSData
            if let Some(v) = value.as_object() {
                let data = v.decode_data("NS.data")?;
                return Ok(data.to_vec());
            }
        }
        Err(DeError::ExpectedData)
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl<T: Decodable> Decodable for Vec<T> {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedObject);
        };
        let obj = value.as_object().ok_or(DeError::ExpectedObject)?;
        /*
        fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSArray"
            || classes[0] == "NSMutableArray"
            || classes[0] == "NSSet"
            || classes[0] == "NSMutableSet"
        }
        */
        /* if !NSArray::is_type_of(obj.classes()) {
            return Err(DeError::Message("NSArray: not an array".to_string()));
        } */
        let Ok(inner_objs) = obj.decode_array("NS.objects") else {
            return Err(DeError::Message(
                "NSArray: Expected array of objects".to_string(),
            ));
        };
        let mut result = Vec::with_capacity(inner_objs.len());
        for inner_obj in inner_objs {
            result.push(T::decode(&ObjectValue::Ref(inner_obj.clone()), types)?);
        }

        /*let arr = NSArray::get_from_object(obj, key, types)?;
        arr.try_into_objects::<T>()*/
        Ok(result)
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for ValueRef {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedObject);
        };
        Ok(value.clone())
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for UniqueId {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedObject);
        };
        Ok(value.unique_id)
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl<T: Decodable> Decodable for Option<T> {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        // None variant is handled in #[derive(Decodable)]
        // Kinda hacky, but it works
        Ok(Some(T::decode(value, types)?))
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for f64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError> {
        if let ObjectValue::Real(value) = value {
            return Ok(*value);
        }
        if let ObjectValue::Ref(value) = value {
            if let Some(v) = value.as_float() {
                return Ok(v);
            }
        }
        Err(DeError::ExpectedFloat)
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for Integer {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: &ObjectValue, _types: &[ObjectType]) -> Result<Self, DeError> {
        if let ObjectValue::Integer(value) = value {
            return Ok(*value);
        }
        if let ObjectValue::Ref(value) = value {
            if let Some(v) = value.as_integer() {
                return Ok(*v);
            }
        }
        Err(DeError::ExpectedInteger)
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for u64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError> {
        let integer = Integer::decode(value, types)?;
        integer.as_unsigned().ok_or(DeError::Message(
            "Unable to represent an integer as u64".into(),
        ))
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

impl Decodable for i64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError> {
        let integer = Integer::decode(value, types)?;
        integer.as_signed().ok_or(DeError::Message(
            "Unable to represent an integer as i64".into(),
        ))
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized + 'static,
    {
        None
    }
}

// TODO: A HashMap key should implement Eq and Hash. It's not possible for any Rust struct,
// so some amount dicts aren't decodable.
impl<K: Decodable + std::hash::Hash + Eq, V: Decodable> Decodable for HashMap<K, V> {
    fn is_type_of(_classes: &[String]) -> bool
    where
        Self: Sized,
    {
        false
    }

    fn class(&self) -> &str {
        ""
    }

    fn decode(value: &ObjectValue, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let obj = as_object!(value)?;
        let raw_keys = obj.decode_array("NS.keys")?;
        let mut keys = Vec::with_capacity(raw_keys.len());
        for key in raw_keys {
            keys.push(K::decode(&key.into(), types)?);
        }
        let mut objects = Vec::<V>::decode(value, types)?;

        if keys.len() != objects.len() {
            return Err(DeError::Message(
                "NSDictionary: The number of keys is not equal to the number of values".to_string(),
            ));
        }
        let mut hashmap = HashMap::with_capacity(keys.len());
        for _ in 0..keys.len() {
            hashmap.insert(keys.pop().unwrap(), objects.pop().unwrap());
        }
        Ok(hashmap)
    }
}
