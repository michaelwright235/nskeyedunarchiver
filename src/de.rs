use std::{cell::Ref, collections::{hash_map::Keys, HashMap}, ops::Index};

use log::debug;
use serde::{de::{self, MapAccess, SeqAccess, Visitor}, Deserialize};
use thiserror::Error;
use crate::{ArchiveValue, Object, ObjectRef, ObjectValue};

#[derive(Debug, Error)]
pub enum DeError {
    #[error("{0}")]
    Message(String),

    // Zero or more variants that can be created directly by the Serializer and
    // Deserializer without going through `ser::Error` and `de::Error`. These
    // are specific to the format, in this case JSON.
    #[error("Syntax error")]
    Syntax,
    #[error("Expected boolean")]
    ExpectedBoolean,
    #[error("Expected signed integer")]
    ExpectedSignedInteger,
    #[error("Expected unsigned integer")]
    ExpectedUnsignedInteger,
    #[error("Expected float")]
    ExpectedFloat,
    #[error("Expected string")]
    ExpectedString,
    #[error("Expected null")]
    ExpectedNull,
    #[error("Expected array")]
    ExpectedArray,
    #[error("Expected dictionary")]
    ExpectedDictionary,
    #[error("Expected data")]
    ExpectedData,
    #[error("Expected object")]
    ExpectedObject,
    #[error("Unknown class {0}")]
    UnknownClass(String),
    #[error("Unsupported value type")]
    UnsupportedType
}

impl de::Error for DeError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        DeError::Message(msg.to_string())
    }
}

////////

struct ObjectMapAccessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    currect_item: usize
}

impl<'a, 'de> ObjectMapAccessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self {
            de,
            currect_item: 0,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for ObjectMapAccessor<'a, 'de> {
    type Error = DeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de> {
        let borrowed = &*self.de.object;
        let obj = borrowed.as_object().unwrap();
        let Some((key, _)) = obj.fields.iter().nth(self.currect_item) else {
            self.de.current_key = None;
            return Ok(None);
        };
        self.de.current_key = Some(key.to_string());
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de> {
            let borrowed = &*self.de.object;
            let obj = borrowed.as_object().unwrap();
            let (_, vec) = obj.fields.iter().nth(self.currect_item).unwrap();
            self.de.current_value = Some(vec.clone());
            let result = seed.deserialize(&mut *self.de);
            debug!("ObjectMapAccessor: {:?}", result.is_ok());
            self.currect_item +=1;
            result
    }

}

struct SeqAccessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    seq_len: usize,
}

impl<'a, 'de> SeqAccessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, seq_len: usize) -> Self {
        Self {
            de,
            seq_len,
        }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqAccessor<'a, 'de> {
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        debug!("current value = {:?}", self.de.current_value);
        if self.de.seq_counter >= self.seq_len {
            return Ok(None);
        }
        let result = seed.deserialize(&mut *self.de).map(Some);
        self.de.seq_counter +=1;
        result
    }
}

////////

pub struct Deserializer<'de> {
    object: &'de ObjectRef,
    current_key: Option<String>,
    current_value: Option<ObjectValue>,
    seq_counter: usize,
}

impl<'de> Deserializer<'de> {
    pub fn from_object(object: &'de ObjectRef) -> Self {
        log::debug!("from_object");
        Deserializer { object: object, current_key: None, current_value: None, seq_counter: 0 }
    }
}

pub fn from_object<'a, T>(object: &'a ObjectRef) -> Result<T, DeError>
where
    T: Deserialize<'a>,
{
    log::debug!("fn from_object");
    let mut deserializer = Deserializer::from_object(object);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = DeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        log::debug!("deserialize_any");
        match self.object.as_ref() {
            crate::ArchiveValue::String(_) => self.deserialize_str(visitor),
            crate::ArchiveValue::Integer(i) => {
                if i.as_signed().is_some() {
                    self.deserialize_i64(visitor)
                } else {
                    self.deserialize_u64(visitor)
                }
            },
            crate::ArchiveValue::F64(_) => self.deserialize_f64(visitor),
            crate::ArchiveValue::NullRef => todo!(),
            crate::ArchiveValue::Classes(_) => todo!(),
            crate::ArchiveValue::Object(obj) => {
                let cls = obj.class();

                //self.deserialize_newtype_struct(&cls, visitor)
                todo!()
            },

            //_ => Err(DeError::Syntax),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        log::debug!("deserialize_bool");
        match self.current_value.as_ref().unwrap() {
            // TODO
            ObjectValue::Boolean(b) => {
                visitor.visit_bool(*b)
            }
            _ => unreachable!()
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        log::debug!("deserialize_i8");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        log::debug!("deserialize_i16");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        log::debug!("deserialize_i32");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_i64");
        match self.current_value.as_ref().unwrap() {
            ObjectValue::Integer(u) => {
                visitor.visit_i64(u.as_signed().unwrap())
            }
            _ => unreachable!()
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_u8");
        match self.current_value.as_ref().unwrap() {
            // TODO
            ObjectValue::Data(d) => {
                visitor.visit_u8(d[self.seq_counter])
            }
            _ => unreachable!()
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_u16");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_u32");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_u64");
        match self.current_value.as_ref().unwrap() {
            ObjectValue::Integer(u) => {
                visitor.visit_u64(u.as_unsigned().unwrap())
            }
            _ => unreachable!()
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_f32");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_f64");
        match self.current_value.as_ref().unwrap() {
            ObjectValue::F64(f) => {
                visitor.visit_f64(*f)
            }
            _ => unreachable!()
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_char");
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_str");
        visitor.visit_str(&self.current_key.as_ref().unwrap())
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        log::debug!("deserialize_string");
        todo!()
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_bytes");
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_byte_buf");
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_option");
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_unit");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_unit_struct");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_newtype_struct");
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_seq");
        let obj = self.current_value.as_ref().unwrap();
        let len = match obj {
            ObjectValue::Data(d) => d.len(),
            ObjectValue::RefArray(a) => a.len(),
            _ => unreachable!()
        };
        visitor.visit_seq(SeqAccessor::new(self, len))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_tuple");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_tuple_struct");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_map");
        Err(DeError::UnsupportedType)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        log::debug!("deserialize_struct");
        debug!("name: {name}, fields: {fields:?}");
        let r  = self.object;
        let Some(obj) = r.as_object() else {
            return Err(DeError::ExpectedObject);
        };
        if name != &obj.class() {
            return Err(DeError::UnknownClass(obj.class()));
        }
        visitor.visit_map(ObjectMapAccessor::new(self))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_enum");
        let array = self.current_value.as_ref().unwrap().as_ref_array().unwrap();
        for item in array {
            //self.object = item;

        }
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        let result = self.deserialize_str(visitor);
        log::debug!("deserialize_identifier: {:?}", result.is_ok());
        result
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            log::debug!("deserialize_ignored_any");
        debug!("current key = {:?}", self.current_key);
        match self.current_value.take().unwrap() {
            crate::ObjectValue::String(s) => visitor.visit_string(s),
            crate::ObjectValue::Integer(n) => {
                if n.as_signed().is_some() {
                    let i = n.as_signed().unwrap();
                    //self.seq = Some(i.to_string().);
                    visitor.visit_i64(i)
                } else {
                    let u = n.as_unsigned().unwrap();
                    //self.seq = Some(u.to_be_bytes().to_vec());
                    visitor.visit_u64(u)
                }
            },
            ObjectValue::Data(v) => {
                visitor.visit_bytes(&v)
            },
            crate::ObjectValue::F64(f) => visitor.visit_f64(f),
            crate::ObjectValue::NullRef => todo!(),
            _ => {
                todo!()
            }
        }
    }
}
