use std::collections::HashMap;

use crate::{DeError, Error, Integer, ValueRef, NULL_OBJECT_REFERENCE_NAME};
use plist::{Dictionary as PlistDictionary, Value as PlistValue};

macro_rules! get_key {
    ($self:ident, $key:ident, $typ:literal) => {{
        if !$self.contains_key($key) {
            return Err(DeError::Message(format!(
                "Missing key '{0}' for object '{1}'",
                $key,
                $self.class()
            )));
        }
        let raw_object = $self.fields.get($key).unwrap();
        paste::paste!{
            let obj = if let ObjectValue::[<$typ:camel>](v) = raw_object {
                Some(v)
            } else {
                None
            };
        }
        if obj.is_none() {
            return Err(DeError::Message(format!(
                "Incorrect value type of '{0}' for object '{1}'. Expected '{2}' for key '{3}'",
                $typ,
                $self.class(),
                raw_object.as_plain_type(),
                $key.to_string()
            )));
        }
        obj.unwrap()
    }};
}

#[derive(Debug, Clone)]
enum ObjectValue {
    String(String),
    Integer(Integer),
    Real(f64),
    Boolean(bool),
    Data(Vec<u8>),
    RefArray(Vec<ValueRef>),
    Ref(ValueRef),
    NullRef,

    // Only used when creataing an object
    RawRefArray(Vec<u64>), // vector of uids
    RawRef(u64),           // uid
}
impl ObjectValue {
    pub fn as_plain_type(&self) -> &'static str {
        match self {
            ObjectValue::String(_) => "string",
            ObjectValue::Integer(_) => "integer",
            ObjectValue::Real(_) => "f64",
            ObjectValue::Boolean(_) => "boolean",
            ObjectValue::Data(_) => "data",
            ObjectValue::RefArray(_) => "array of objects references",
            ObjectValue::Ref(_) => "object reference",
            ObjectValue::NullRef => "null reference",
            ObjectValue::RawRefArray(_) => todo!(),
            ObjectValue::RawRef(_) => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct Object {
    classes: Option<ValueRef>,
    classes_uid: u64,
    fields: HashMap<String, ObjectValue>,
}

impl Object {
    pub fn decode_bool(&self, key: &str) -> Result<bool, DeError> {
        Ok(*get_key!(self, key, "boolean"))
    }

    pub fn decode_data(&self, key: &str) -> Result<&[u8], DeError> {
        Ok(get_key!(self, key, "data"))
    }

    pub fn decode_real(&self, key: &str) -> Result<&f64, DeError> {
        Ok(get_key!(self, key, "real"))
    }

    pub fn decode_integer(&self, key: &str) -> Result<&Integer, DeError> {
        Ok(get_key!(self, key, "integer"))
    }

    pub fn decode_string(&self, key: &str) -> Result<&str, DeError> {
        // As far as I can tell all strings inside of objects are
        // linked with UIDs
        let obj = get_key!(self, key, "ref");
        let Some(string) = obj.as_string() else {
            return Err(DeError::Message(format!(
                "Incorrect value type of '{0}' for object '{1}'. Expected '{2}' for key '{3}'",
                "object",
                self.class(),
                "string",
                key
            )));
        };

        Ok(string)
    }

    pub fn decode_object(&self, key: &str) -> Result<&ValueRef, DeError> {
        let obj = get_key!(self, key, "ref");
        Ok(obj)
    }

    pub fn decode_array(&self, key: &str) -> Result<&[ValueRef], DeError> {
        let array = get_key!(self, key, "ref_array");
        Ok(array)
    }

    pub fn is_null_ref(&self, key: &str) -> Result<bool, DeError> {
        if !self.contains_key(key) {
            return Err(DeError::Message(format!(
                "Missing key '{0}' for object '{1}'",
                self.class(),
                key
            )));
        }
        Ok(
            if let ObjectValue::NullRef = self.fields.get(key).unwrap() {
                true
            } else {
                false
            }
        )
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    pub fn classes(&self) -> &[String] {
        let a = self.classes.as_ref().unwrap();
        let b = a.as_classes().unwrap();
        b
    }

    pub fn class(&self) -> &str {
        let a = self.classes.as_ref().unwrap();
        &a.as_classes().as_ref().unwrap()[0]
    }

    pub(crate) fn apply_value_refs(&mut self, tree: &[ValueRef]) -> Result<(), Error> {
        self.classes = Some(tree[self.classes_uid as usize].clone());
        if !self.classes.as_ref().unwrap().is_classes() {
            return Err(Error::IncorrectFormat(
                format!("Incorrent Classes structure (uid: {})", self.classes_uid)
            ));
        }

        for value in self.fields.values_mut() {
            if let ObjectValue::RawRef(r) = value {
                if let Some(obj_ref) = tree.get(*r as usize) {
                    *value = ObjectValue::Ref(obj_ref.clone());
                } else {
                    return Err(Error::IncorrectFormat(
                        format!("Incorrent object uid: {r}"),
                    ))
                }
            }
            if let ObjectValue::RawRefArray(arr) = value {
                let mut ref_arr = Vec::with_capacity(arr.len());
                for item in arr {
                    if let Some(obj_ref) = tree.get(*item as usize) {
                        ref_arr.push(obj_ref.clone())
                    } else {
                        return Err(Error::IncorrectFormat(
                            format!("Incorrent object uid: {item}"),
                        ))
                    }
                }
                *value = ObjectValue::RefArray(ref_arr);
            }
        }
        Ok(())
    }

    pub(crate) fn from_dict(mut dict: PlistDictionary) -> Result<Self, Error> {
        // unwrapping is safe, we previously check it with is_container()
        let classes_uid = dict.remove("$class").unwrap().into_uid().unwrap().get();
        let mut fields = HashMap::with_capacity(dict.len());
        for (key, obj) in dict {
            let decoded_obj = if let Some(s) = obj.as_string() {
                if s == NULL_OBJECT_REFERENCE_NAME {
                    ObjectValue::NullRef
                } else {
                    ObjectValue::String(obj.into_string().unwrap())
                }
            } else if let PlistValue::Integer(i) = obj {
                ObjectValue::Integer(i)
            } else if let Some(f) = obj.as_real() {
                ObjectValue::Real(f)
            } else if let Some(b) = obj.as_boolean() {
                ObjectValue::Boolean(b)
            } else if obj.as_data().is_some() {
                ObjectValue::Data(obj.into_data().unwrap())
            } else if let Some(arr) = obj.as_array() {
                let mut arr_of_uids = Vec::with_capacity(arr.len());
                for val in obj.into_array().unwrap() {
                    if val.as_uid().is_none() {
                        return Err(Error::IncorrectFormat(
                            format!("Array (uid: {classes_uid}) should countain only object references"),
                        ));
                    } else {
                        arr_of_uids.push(val.into_uid().unwrap().get());
                    }
                }
                ObjectValue::RawRefArray(arr_of_uids)
            } else if obj.as_uid().is_some() {
                ObjectValue::RawRef(obj.into_uid().unwrap().get())
            } else {
                return Err(Error::IncorrectFormat(format!(
                    "Enexpected object (uid: {classes_uid}) value type: {:?}",
                    obj
                )));
            };
            fields.insert(key, decoded_obj);
        }
        Ok(Self {
            classes: None,
            classes_uid,
            fields,
        })
    }
}
