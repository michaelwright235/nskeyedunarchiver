use super::{value_ref_to_any, Decodable, ObjectType};
use crate::{as_object, DeError, Integer, ValueRef};
use std::collections::HashMap;

impl Decodable for String {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSString" || classes[0] == "NSMutableString"
    }
    fn class(&self) -> &str {
        "NSString"
    }
    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        // A string can be encoded as a plain String type
        if let Some(s) = value.as_string() {
            return Ok(s.to_string());
        }

        // ... or as an Object with `NS.bytes` data or `NS.string` string (NIB Archives)
        if value.as_object().is_none() {
            return Err(DeError::ExpectedString);
        }

        let obj = value.as_object().unwrap();
        if !obj.contains_key("NS.bytes") && !obj.contains_key("NS.string") {
            return Err(DeError::ExpectedString);
        }
        let s = if let Ok(data) = obj.decode_data("NS.bytes") {
            let parsed = String::from_utf8(data.to_vec());
            if let Err(e) = parsed {
                return Err(DeError::Message(format!(
                    "Unable to parse a UTF-8 string: {e}"
                )));
            }
            parsed.unwrap()
        } else if let Ok(data) = obj.decode_string("NS.string") {
            data.into_owned()
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

    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Ok(value.as_boolean().ok_or(DeError::ExpectedBoolean)?)
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

    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Ok(value.as_data().ok_or(DeError::ExpectedData)?.to_vec())
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

    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        let obj = value.as_object().ok_or(DeError::ExpectedObject)?;
        if !NSArray::is_type_of(obj.classes()) {
            return Err(DeError::Message("NSArray: not an array".to_string()));
        }
        let Ok(inner_objs) = obj.decode_array("NS.objects") else {
            return Err(DeError::Message(
                "NSArray: Expected array of objects".to_string(),
            ));
        };
        let mut result = Vec::with_capacity(inner_objs.len());
        for inner_obj in inner_objs {
            result.push(T::decode(inner_obj.clone(), types)?);
        }

        /*let arr = NSArray::get_from_object(obj, key, types)?;
        arr.try_into_objects::<T>()*/
        Ok(result)
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

    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Ok(value)
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

    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        Ok(T::decode(value, types).ok())
    }
}

impl Decodable for f64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(float) = value.as_float() else {
            return Err(DeError::ExpectedFloat);
        };
        Ok(float)
    }
}

impl Decodable for Integer {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(int) = value.as_integer() else {
            return Err(DeError::ExpectedInteger);
        };
        Ok(*int)
    }
}

impl Decodable for u64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let integer = Integer::decode(value, types)?;
        Ok(integer.as_unsigned().ok_or(DeError::Message(
            "Unable to represent an integer as u64".into(),
        ))?)
    }
}

impl Decodable for i64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn class(&self) -> &str {
        ""
    }
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let integer = Integer::decode(value, types)?;
        Ok(integer.as_signed().ok_or(DeError::Message(
            "Unable to represent an integer as i64".into(),
        ))?)
    }
}

macro_rules! class_wrapper {
    ($name:ident, $dataType:ty) => {
        impl $name {
            pub fn new(data: $dataType) -> Self {
                Self {
                    data,
                    is_mutable: false,
                }
            }
            pub fn new_mut(data: $dataType) -> Self {
                Self {
                    data,
                    is_mutable: true,
                }
            }
            pub fn set_is_mutable(&mut self, v: bool) {
                self.is_mutable = v;
            }
            pub fn is_mutable(&self) -> bool {
                self.is_mutable
            }
            pub fn into_inner(self) -> $dataType {
                self.data
            }
        }

        impl AsRef<$dataType> for $name {
            fn as_ref(&self) -> &$dataType {
                &self.data
            }
        }

        impl AsMut<$dataType> for $name {
            fn as_mut(&mut self) -> &mut $dataType {
                &mut self.data
            }
        }
    };
}

#[derive(Debug)]
pub struct NSArray {
    data: Vec<Box<dyn Decodable>>,
    is_mutable: bool,
}
class_wrapper!(NSArray, Vec<Box<dyn Decodable>>);

impl Decodable for NSArray {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSArray"
            || classes[0] == "NSMutableArray"
            || classes[0] == "NSSet"
            || classes[0] == "NSMutableSet"
    }
    fn class(&self) -> &str {
        if !self.is_mutable {
            "NSArray"
        } else {
            "NSMutableArray"
        }
    }
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(value)?;
        let is_mutable = obj.class() == "NSMutableArray";
        let Ok(inner_objs) = obj.decode_array("NS.objects") else {
            return Err(DeError::Message(
                "NSArray: Expected array of objects".to_string(),
            ));
        };

        // TODO: add data
        let mut decoded_objs = Vec::with_capacity(inner_objs.len());
        for obj in inner_objs {
            if obj.as_ref().as_string().is_some() {
                let s = String::decode(obj.clone(), &[])?;
                decoded_objs.push(Box::new(s) as Box<dyn Decodable>);
            } else if obj.as_ref().as_integer().is_some() {
                let i = Integer::decode(obj.clone(), &[])?;
                decoded_objs.push(Box::new(i) as Box<dyn Decodable>);
            } else if obj.as_ref().as_float().is_some() {
                let f = f64::decode(obj.clone(), &[])?;
                decoded_objs.push(Box::new(f) as Box<dyn Decodable>);
            } else if obj.as_ref().as_object().is_some() {
                decoded_objs.push(value_ref_to_any(obj.clone(), types)?);
            }
        }

        Ok(Self {
            data: decoded_objs,
            is_mutable,
        })
    }
}

impl NSArray {
    pub fn try_into_objects<T>(self) -> Result<Vec<T>, DeError>
    where
        T: Decodable + 'static,
    {
        let data = self.data;
        for value in &data {
            if value.downcast_ref::<T>().is_none() {
                return Err(DeError::DowncastError);
            }
        }
        let mut objects: Vec<T> = Vec::with_capacity(data.len());
        for obj in data {
            let downcasted = obj.downcast::<T>().unwrap();
            objects.push(*downcasted);
        }
        Ok(objects)
    }
    pub fn get_as_object<T>(&self, index: usize) -> Result<&T, DeError>
    where
        T: Decodable + 'static,
    {
        if self.data.get(index).is_none() {
            return Err(DeError::Message("NSArray: Missing array key".to_string()));
        };
        let Some(downcasted) = self.data.get(index).unwrap().downcast_ref::<T>() else {
            return Err(DeError::DowncastError);
        };
        Ok(downcasted)
    }
    pub fn remove_as_object<T>(&mut self, index: usize) -> Result<T, DeError>
    where
        T: Decodable + 'static,
    {
        let _ = self.get_as_object::<T>(index)?;
        let downcasted = self.data.remove(index).downcast::<T>().unwrap();
        Ok(*downcasted)
    }
}

#[derive(Debug)]
pub struct NSSet {
    data: Vec<Box<dyn Decodable>>,
    is_mutable: bool,
}
impl Decodable for NSSet {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSSet" || classes[0] == "NSMutableSet"
    }
    fn class(&self) -> &str {
        if !self.is_mutable {
            "NSSet"
        } else {
            "NSMutableSet"
        }
    }
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(value)?;
        let is_mutable = obj.class() == "NSMutableSet";
        Ok(Self {
            data: NSArray::decode(value, types)?.into_inner(),
            is_mutable,
        })
    }
}
class_wrapper!(NSSet, Vec<Box<dyn Decodable>>);

impl From<NSArray> for NSSet {
    fn from(value: NSArray) -> Self {
        Self {
            data: value.data,
            is_mutable: value.is_mutable,
        }
    }
}

impl From<NSSet> for NSArray {
    fn from(value: NSSet) -> Self {
        Self {
            data: value.data,
            is_mutable: value.is_mutable,
        }
    }
}

#[derive(Debug)]
pub struct NSDictionary {
    data: HashMap<String, Box<dyn Decodable>>,
    is_mutable: bool,
}

impl Decodable for NSDictionary {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSDictionary" || classes[0] == "NSMutableDictionary"
    }
    fn class(&self) -> &str {
        if !self.is_mutable {
            "NSDictionary"
        } else {
            "NSMutableDictionary"
        }
    }
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(value)?;
        let is_mutable = obj.class() == "NSMutableDictionary";
        let raw_keys = obj.decode_array("NS.keys")?;
        let mut keys = Vec::with_capacity(raw_keys.len());
        for key in raw_keys {
            let Some(name) = key.as_string() else {
                return Err(DeError::Message(
                    "NSDictionary: Key is not a string".to_string(),
                ));
            };
            keys.push(name.to_string());
        }
        let mut objects = NSArray::decode(value, types)?;

        if keys.len() != objects.as_ref().len() {
            return Err(DeError::Message(
                "NSDictionary: The number of keys is not equal to the number of values".to_string(),
            ));
        }
        let mut hashmap = HashMap::with_capacity(keys.len());
        for _ in 0..keys.len() {
            hashmap.insert(keys.pop().unwrap(), objects.as_mut().pop().unwrap());
        }
        Ok(Self {
            data: hashmap,
            is_mutable,
        })
    }
}
class_wrapper!(NSDictionary, HashMap<String, Box<dyn Decodable>>);

impl NSDictionary {
    pub fn try_into_objects<T>(self) -> Result<HashMap<String, Box<T>>, DeError>
    where
        T: Decodable + 'static,
    {
        let data = self.data;
        for value in data.values() {
            if value.downcast_ref::<T>().is_none() {
                return Err(DeError::DowncastError);
            }
        }

        let mut objects: HashMap<String, Box<T>> = HashMap::with_capacity(data.len());
        for (key, value) in data {
            let downcasted = value.downcast::<T>().unwrap();
            objects.insert(key, downcasted);
        }
        Ok(objects)
    }

    pub fn remove_as_object<T>(&mut self, key: &str) -> Result<Box<T>, DeError>
    where
        T: Decodable + 'static,
    {
        let _ = self.get_as_object::<T>(key)?;
        let downcasted = self.data.remove(key).unwrap().downcast::<T>().unwrap();
        Ok(downcasted)
    }

    pub fn get_as_object<T>(&self, key: &str) -> Result<&T, DeError>
    where
        T: Decodable + 'static,
    {
        if !self.data.contains_key(key) {
            return Err(DeError::Message(
                "NSDictionary: Missing hashmap key".to_string(),
            ));
        };

        let Some(downcasted) = self.data.get(key).unwrap().downcast_ref::<T>() else {
            return Err(DeError::DowncastError);
        };
        Ok(downcasted)
    }
}

#[derive(Debug)]
pub struct NSData {
    data: Vec<u8>,
    is_mutable: bool,
}
class_wrapper!(NSData, Vec<u8>);

impl Decodable for NSData {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSData" || classes[0] == "NSMutableData"
    }
    fn class(&self) -> &str {
        if !self.is_mutable {
            "NSData"
        } else {
            "NSMutableData"
        }
    }
    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(value)?;
        let is_mutable = obj.class() == "NSMutableData";
        let data = obj.decode_data("NS.data")?.to_vec();
        Ok(Self { data, is_mutable })
    }
}
