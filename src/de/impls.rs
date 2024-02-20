use super::{object_ref_to_any, Decodable, ObjectType};
use crate::{as_object, ArchiveValue, DeError, ObjectRef};
use std::any::Any;
use std::collections::HashMap;

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

impl Decodable for u64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(n) = object.as_integer() else {
            return Err(DeError::ExpectedInteger);
        };
        let Some(unsigned) = n.as_unsigned() else {
            return Err(DeError::Message("Expected unsigned integer".to_string()));
        };
        Ok(unsigned)
    }
}

impl Decodable for i64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(n) = object.as_integer() else {
            return Err(DeError::ExpectedInteger);
        };
        let Some(signed) = n.as_signed() else {
            return Err(DeError::Message("Expected signed integer".to_string()));
        };
        Ok(signed)
    }
}

impl Decodable for f64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(float) = object.as_f64() else {
            return Err(DeError::ExpectedInteger);
        };
        Ok(*float)
    }
}

#[derive(Debug)]
pub struct NSArray {
    objects: Vec<Box<dyn Any>>, // NS.objects
}

impl Decodable for NSArray {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSArray"
    }
    fn decode(object: ObjectRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(object);
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
                    if n.as_signed().is_some() {
                        let i = i64::decode(obj.clone(), &[])?;
                        decoded_objs.push(Box::new(i) as Box<dyn Any>);
                    } else {
                        let u = u64::decode(obj.clone(), &[])?;
                        decoded_objs.push(Box::new(u) as Box<dyn Any>);
                    }
                }
                ArchiveValue::F64(_) => {
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

        Ok(Self {
            objects: decoded_objs,
        })
    }
}

impl NSArray {
    pub fn objects(&self) -> &[Box<dyn Any>] {
        &self.objects
    }
    pub fn into_inner(self) -> Vec<Box<dyn Any>> {
        self.objects
    }
}

#[derive(Debug)]
pub struct NSDictionary {
    hashmap: HashMap<String, Box<dyn Any>>,
}

impl Decodable for NSDictionary {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSDictionary"
    }

    fn decode(object: ObjectRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(object);
        let raw_keys = obj.decode_array("NS.keys")?;
        let mut keys = Vec::with_capacity(raw_keys.len());
        for key in raw_keys {
            let Some(name) = key.as_string() else {
                return Err(DeError::ExpectedString);
            };
            keys.push(name.to_string());
        }
        let mut objects = NSArray::decode(object, types)?.into_inner();

        if keys.len() != objects.len() {
            return Err(DeError::Message(
                "NSDictionary: The number of keys is not equal to the number of values".to_string(),
            ));
        }
        let mut hashmap = HashMap::with_capacity(keys.len());
        for _ in 0..keys.len() {
            hashmap.insert(keys.pop().unwrap(), objects.pop().unwrap());
        }
        Ok(Self { hashmap })
    }
}

impl NSDictionary {
    pub fn hashmap(&self) -> &HashMap<String, Box<dyn Any>> {
        &self.hashmap
    }
    pub fn into_inner(self) -> HashMap<String, Box<dyn Any>> {
        self.hashmap
    }
}
