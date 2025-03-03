use super::Decodable;
use crate::{DeError, Integer, Object, ObjectValue, UniqueId, ValueRef};
use std::collections::HashMap;

impl Decodable for String {
    fn decode(value: &ObjectValue) -> Result<Self, DeError> {
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
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
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
}

impl Decodable for Vec<u8> {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
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
}

/// Decodes NS.objects array to a vector of decodables.
/// Used by Vec and Hashmap.
fn refs_to_t<T: Decodable>(obj: &Object) -> Result<Vec<T>, DeError> {
    let Ok(inner_objs) = obj.decode_array("NS.objects") else {
        return Err(DeError::Message(
            "NSArray: Missing NS.objects key".to_string(),
        ));
    };
    let mut result = Vec::with_capacity(inner_objs.len());
    for inner_obj in inner_objs {
        result.push(T::decode(&ObjectValue::Ref(inner_obj.clone()))?);
    }
    Ok(result)
}

impl<T: Decodable> Decodable for Vec<T> {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedObject);
        };
        let obj = value.as_object().ok_or(DeError::ExpectedObject)?;

        if obj.class() != "NSArray"
            && obj.class() != "NSMutableArray"
            && obj.class() != "NSSet"
            && obj.class() != "NSMutableSet"
        {
            return Err(DeError::Message("NSArray: not an array".to_string()));
        }

        refs_to_t(obj)
    }
}

impl Decodable for ValueRef {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedObject);
        };
        Ok(value.clone())
    }
}

impl Decodable for UniqueId {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(value) = value else {
            return Err(DeError::ExpectedObject);
        };
        Ok(value.unique_id)
    }
}

impl<T: Decodable> Decodable for Option<T> {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        // None variant is handled in #[derive(Decodable)]
        // Kinda hacky, but it works
        Ok(Some(T::decode(value)?))
    }
}

impl Decodable for f64 {
    fn decode(value: &ObjectValue) -> Result<Self, DeError> {
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
}

impl Decodable for Integer {
    fn decode(value: &ObjectValue) -> Result<Self, DeError> {
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
}

impl Decodable for u64 {
    fn decode(value: &ObjectValue) -> Result<Self, DeError> {
        let integer = Integer::decode(value)?;
        integer.as_unsigned().ok_or(DeError::Message(
            "Unable to represent an integer as u64".into(),
        ))
    }
}

impl Decodable for i64 {
    fn decode(value: &ObjectValue) -> Result<Self, DeError> {
        let integer = Integer::decode(value)?;
        integer.as_signed().ok_or(DeError::Message(
            "Unable to represent an integer as i64".into(),
        ))
    }
}

// TODO: A HashMap key should implement Eq and Hash. It's not possible for any Rust struct,
// so some amount of dicts aren't decodable. Usually a key is a String anyway.
impl<K: Decodable + std::hash::Hash + Eq, V: Decodable> Decodable for HashMap<K, V> {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let ObjectValue::Ref(obj_value) = value else {
            return Err(DeError::ExpectedObject);
        };
        let obj = obj_value.as_object().ok_or(DeError::ExpectedObject)?;

        if obj.class() != "NSDictionary" && obj.class() != "NSMutableDictionary" {
            return Err(DeError::Message(
                "NSDictionary: not a dictionary".to_string(),
            ));
        }

        let raw_keys = obj.decode_array("NS.keys")?;
        let mut keys = Vec::with_capacity(raw_keys.len());
        for key in raw_keys {
            keys.push(K::decode(&key.into())?);
        }

        let mut objects = refs_to_t(obj)?;

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
