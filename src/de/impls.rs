use super::{value_ref_to_any, Decodable, ObjectType};
use crate::{as_object, DeError, Integer, ValueRef};
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

impl Decodable for String {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(s) = value.as_string() else {
            return Err(DeError::ExpectedString);
        };
        Ok(s.to_string())
    }
}

impl Decodable for f64 {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }
    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(float) = value.as_real() else {
            return Err(DeError::ExpectedReal);
        };
        Ok(*float)
    }
}

impl Decodable for Integer {
    fn is_type_of(_classes: &[String]) -> bool {
        false
    }

    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let Some(int) = value.as_integer() else {
            return Err(DeError::ExpectedInteger);
        };
        Ok(*int)
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
    is_mutable: bool,
}
class_wrapper!(NSArray, Vec<Box<dyn Any>>);

impl Decodable for NSArray {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSArray"
            || classes[0] == "NSMutableArray"
            || classes[0] == "NSSet"
            || classes[0] == "NSMutableSet"
    }
    fn decode(value: ValueRef, types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(value)?;
        let is_mutable = obj.class() == "NSMutableArray";
        let Ok(inner_objs) = obj.decode_array("NS.objects") else {
            return Err(DeError::Message(
                "NSArray: Expected array of objects".to_string(),
            ));
        };

        let mut decoded_objs = Vec::with_capacity(inner_objs.len());
        for obj in inner_objs {
            if let Some(_) = obj.as_ref().as_string() {
                let s = String::decode(obj.clone(), &[])?;
                decoded_objs.push(Box::new(s) as Box<dyn Any>);
            } else if let Some(_) = obj.as_ref().as_integer() {
                let i = Integer::decode(obj.clone(), &[])?;
                decoded_objs.push(Box::new(i) as Box<dyn Any>);
            } else if let Some(_) = obj.as_ref().as_real() {
                let f = f64::decode(obj.clone(), &[])?;
                decoded_objs.push(Box::new(f) as Box<dyn Any>);
            } else if let Some(_) = obj.as_ref().as_object() {
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
    pub fn try_into_objects<T>(self) -> Result<Vec<Box<T>>, DeError>
    where
        T: Decodable + 'static,
    {
        let data = self.data;
        for value in &data {
            if value.downcast_ref::<T>().is_none() {
                return Err(DeError::Message("NSArray: Unable to downcast objects".to_string()));
            }
        }
        let mut objects: Vec<Box<T>> = Vec::with_capacity(data.len());
        for obj in data {
            let downcasted = obj.downcast::<T>().unwrap();
            objects.push(downcasted);
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
            return Err(DeError::Message("NSArray: Unable to downcast objects".to_string()));
        };
        Ok(downcasted)
    }
    pub fn remove_as_object<T>(&mut self, index: usize) -> Result<Box<T>, DeError>
    where
        T: Decodable + 'static,
    {
        let _ = self.get_as_object::<T>(index)?;
        let downcasted = self.data.remove(index).downcast::<T>().unwrap();
        Ok(downcasted)
    }
}

pub struct NSSet {
    data: Vec<Box<dyn Any>>,
    is_mutable: bool,
}
impl Decodable for NSSet {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSSet" || classes[0] == "NSMutableSet"
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
class_wrapper!(NSSet, Vec<Box<dyn Any>>);

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

pub struct NSDictionary {
    data: HashMap<String, Box<dyn Any>>,
    is_mutable: bool,
}

impl Decodable for NSDictionary {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSDictionary" || classes[0] == "NSMutableDictionary"
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
            is_mutable,
        })
    }
}
class_wrapper!(NSDictionary, HashMap<String, Box<dyn Any>>);

impl NSDictionary {
    pub fn try_into_objects<T>(self) -> Result<HashMap<String, Box<T>>, DeError>
    where
        T: Decodable + 'static,
    {
        let data = self.data;
        for (_, value) in &data {
            if value.downcast_ref::<T>().is_none() {
                return Err(DeError::Message("NSDictionary: Unable to downcast objects".to_string()));
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
        if self.data.get(key).is_none() {
            return Err(DeError::Message("NSDictionary: Missing hashmap key".to_string()));
        };
        let Some(downcasted) = self.data.get(key).unwrap().downcast_ref::<T>() else {
            return Err(DeError::Message("NSDictionary: Unable to downcast objects".to_string()));
        };
        Ok(downcasted)
    }
}

pub struct NSData {
    data: Vec<u8>,
    is_mutable: bool,
}
class_wrapper!(NSData, Vec<u8>);

impl Decodable for NSData {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSData" || classes[0] == "NSMutableData"
    }

    fn decode(value: ValueRef, _types: &[ObjectType]) -> Result<Self, DeError> {
        let obj = as_object!(value)?;
        let is_mutable = obj.class() == "NSMutableData";
        let data = obj.decode_data("NS.data")?.to_vec();
        Ok(Self { data, is_mutable })
    }
}
