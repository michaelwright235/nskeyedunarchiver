use super::{object_ref_to_any, Decodable, ObjectType};
use crate::{as_object, ArchiveValue, DeError, ObjectRef, Integer};
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

impl Decodable for String {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(s) = object.as_string() else {
            return Err(DeError::ExpectedString);
        };
        Ok(s.to_string())
    }
}

impl Decodable for f64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(float) = object.as_real() else {
            return Err(DeError::ExpectedReal);
        };
        Ok(*float)
    }
}

impl Decodable for Integer {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }

    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(int) = object.as_integer() else {
            return Err(DeError::ExpectedInteger);
        };
        Ok(*int)
    }
}

macro_rules! class_wrapper {
    ($name:ident, $dataType:ty) => {
        impl $name {
            pub fn new(data: $dataType) -> Self {
                Self{data, is_mutable: false}
            }
            pub fn new_mut(data: $dataType) -> Self {
                Self{data, is_mutable: true}
            }
            pub fn set_is_mutable(&mut self, v: bool) {
                self.is_mutable = v;
            }
            pub fn is_mutable(&mut self) -> bool {
                self.is_mutable
            }
            pub fn into_inner(self) -> $dataType {
                self.data
            }
        }
        impl Deref for $name {
            type Target = $dataType;

            fn deref(&self) -> &Self::Target {
                &self.data
            }
        }
        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.data
            }
        }
    };
}

pub struct NSArray {
    data: Vec<Box<dyn Any>>,
    is_mutable: bool
}

impl Decodable for NSArray {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSArray" || classes[0] == "NSMutableArray" ||
        classes[0] == "NSSet" || classes[0] == "NSMutableSet"
    }
    fn decode(object: ObjectRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(object);
        let is_mutable = obj.class() == "NSMutableArray";
        let Ok(inner_objs) = obj.decode_array("NS.objects") else {
            return Err(DeError::Message(
                "NSArray: Expected array of objects".to_string(),
            ));
        };

        let mut decoded_objs = Vec::with_capacity(inner_objs.len());
        for obj in inner_objs {
            match obj.as_ref() {
                ArchiveValue::String(_) => {
                    let s = String::decode(obj.clone(), &[])?;
                    decoded_objs.push(Box::new(s) as Box<dyn Any>);
                }
                ArchiveValue::Integer(n) => {
                    decoded_objs.push(Box::new(*n) as Box<dyn Any>);
                }
                ArchiveValue::Real(_) => {
                    let f = f64::decode(obj.clone(), &[])?;
                    decoded_objs.push(Box::new(f) as Box<dyn Any>);
                }
                ArchiveValue::Object(_) => {
                    decoded_objs.push(object_ref_to_any(obj, types)?);
                }
                ArchiveValue::NullRef => (),
                ArchiveValue::Classes(_) => (),
            }
        }

        Ok(Self{
            data: decoded_objs,
            is_mutable
        })
    }
}
class_wrapper!(NSArray, Vec<Box<dyn Any>>);

pub struct NSSet {
    data: Vec<Box<dyn Any>>,
    is_mutable: bool
}
impl Decodable for NSSet {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSSet" || classes[0] == "NSMutableSet"
    }

    fn decode(object: ObjectRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(object);
        let is_mutable = obj.class() == "NSMutableSet";
        Ok(Self {
            data: NSArray::decode(object, types)?.into_inner(),
            is_mutable
        })
    }
}
class_wrapper!(NSSet, Vec<Box<dyn Any>>);

pub struct NSDictionary {
    data: HashMap<String, Box<dyn Any>>,
    is_mutable: bool
}

impl Decodable for NSDictionary {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSDictionary" || classes[0] == "NSMutableDictionary"
    }

    fn decode(object: ObjectRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(object);
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
        let mut objects = NSArray::decode(object, types)?;

        if keys.len() != objects.len() {
            return Err(DeError::Message(
                "NSDictionary: The number of keys is not equal to the number of values".to_string(),
            ));
        }
        let mut hashmap = HashMap::with_capacity(keys.len());
        for _ in 0..keys.len() {
            hashmap.insert(keys.pop().unwrap(), objects.pop().unwrap());
        }
        Ok(Self {
            data: hashmap,
            is_mutable
        })
    }
}
class_wrapper!(NSDictionary, HashMap<String, Box<dyn Any>>);

pub struct NSData {
    data: Vec<u8>,
    is_mutable: bool
}
class_wrapper!(NSData, Vec<u8>);

impl Decodable for NSData {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSData" || classes[0] == "NSMutableData"
    }

    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(object);
        let is_mutable = obj.class() == "NSMutableData";
        let data = obj.decode_data("NS.data")?.to_vec();
        Ok(Self{
            data,
            is_mutable
        })
    }
}
