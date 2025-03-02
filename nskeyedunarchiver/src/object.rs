use std::collections::HashMap;

use crate::{
    de::{value_ref_to_any, Decodable, ObjectType},
    DeError, Error, Integer, ValueRef, NULL_OBJECT_REFERENCE_NAME,
};
use plist::{Dictionary as PlistDictionary, Value as PlistValue};

macro_rules! get_key {
    ($self:ident, $key:ident, $typ:literal) => {{
        if !$self.contains_key($key) {
            return Err(
                DeError::MissingObjectKey($self.class().into(), $key.into())
        );
        }
        let raw_object = $self.fields.get($key).unwrap();
        paste::paste! {
            let obj = if let ObjectValue::[<$typ:camel>](v) = raw_object {
                Some(v)
            } else {
                None
            };
        }
        if obj.is_none() {
            return Err(DeError::Message(format!(
                "{1}: Incorrect value type for key '{3}'. Expected '{0}', found '{2}'",
                $typ,
                $self.class(),
                raw_object.as_plain_type(),
                $key.to_string()
            )));
        }
        obj.unwrap()
    }};
}

#[derive(Debug, PartialEq, Clone)]
enum UninitRefs {
    RawRefArray(Vec<u64>), // vector of uids
    RawRef(u64),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectValue {
    String(String),
    Integer(Integer),
    Real(f64),
    Boolean(bool),
    Data(Vec<u8>),
    RefArray(Vec<ValueRef>),
    Ref(ValueRef),
    NullRef,
}

impl ObjectValue {
    pub fn as_plain_type(&self) -> &'static str {
        match self {
            ObjectValue::String(_) => "string",
            ObjectValue::Integer(_) => "integer",
            ObjectValue::Real(_) => "f64",
            ObjectValue::Boolean(_) => "boolean",
            ObjectValue::Data(_) => "data",
            ObjectValue::RefArray(_) => "array of object references",
            ObjectValue::Ref(_) => "object reference",
            ObjectValue::NullRef => "null reference",
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Object {
    classes: Option<ValueRef>,
    classes_uid: u64,
    fields: HashMap<String, ObjectValue>,
    uninit_fields: Option<HashMap<String, UninitRefs>>,
}

impl Object {
    /// Tries to decode a value as a boolean with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    pub fn decode_bool(&self, key: &str) -> Result<bool, DeError> {
        Ok(*get_key!(self, key, "boolean"))
    }

    /// Tries to decode a value as a data (a vector of bytes) with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    pub fn decode_data(&self, key: &str) -> Result<&[u8], DeError> {
        // In rare cases data may be encoded with a reference
        if let Some(ObjectValue::Ref(obj_ref)) = self.fields.get(key) {
            if let Some(d) = obj_ref.as_data() {
                return Ok(d);
            }
        }
        Ok(get_key!(self, key, "data"))
    }

    /// Tries to decode a value as a float with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    pub fn decode_float(&self, key: &str) -> Result<&f64, DeError> {
        Ok(get_key!(self, key, "real"))
    }

    /// Tries to decode a value as an integer with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    pub fn decode_integer(&self, key: &str) -> Result<&Integer, DeError> {
        Ok(get_key!(self, key, "integer"))
    }

    /// Tries to decode a value as a string with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    ///
    /// NSKeyedArchive objects don't contain plain strings, rather
    /// references to a string value. This function just makes it easy to access.
    pub fn decode_string(&self, key: &str) -> Result<String, DeError> {
        let obj = get_key!(self, key, "ref");
        String::decode(obj.clone(), &vec![])
    }

    /// Tries to decode a value as an object with a given `key` and returns a
    /// [ValueRef] of an archive value.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    ///
    /// One may rarely use this method, look at [decode_object_as] method instead.
    pub fn decode_object(&self, key: &str) -> Result<ValueRef, DeError> {
        let obj = get_key!(self, key, "ref").clone();
        Ok(obj)
    }

    /// Tries to decode a value as a `<T>` object with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    pub fn decode_object_as<T>(&self, key: &str, types: &[ObjectType]) -> Result<T, DeError>
    where
        T: Decodable + 'static,
    {
        let obj = value_ref_to_any(self.decode_object(key)?.clone(), types)?;
        if obj.downcast_ref::<T>().is_none() {
            return Err(DeError::Message(format!(
                "{}: Unable to downcast object '{key}' of class '{}'",
                self.class(),
                obj.class()
            )));
        }
        Ok(*obj.downcast::<T>().unwrap())
    }

    /// Tries to decode a value as an array of value references with a given `key`.
    /// If it doesn't exist or has some other type a [DeError] is returned.
    pub fn decode_array(&self, key: &str) -> Result<&[ValueRef], DeError> {
        let array = get_key!(self, key, "ref_array");
        Ok(array)
    }

    /// Returns the number of object's keys.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns `true` if the object contains no elements.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns an array of object's keys.
    pub fn keys(&self) -> Vec<&String> {
        self.fields.keys().collect()
    }

    /// Checks if a value under the `key` is a null reference.
    /// Returns a [DeError] if a value doesn't exist.
    pub fn is_null_ref(&self, key: &str) -> Result<bool, DeError> {
        if !self.contains_key(key) {
            return Err(
                DeError::MissingObjectKey(self.class().into(), key.into())
            );
        }
        Ok(matches!(
            self.fields.get(key).unwrap(),
            ObjectValue::NullRef
        ))
    }

    /// Checks if the object contains a value with a given `key`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    pub fn as_map(&self) -> &HashMap<String, ObjectValue> {
        &self.fields
    }

    /// Returns classes of the object. The first one is the actual class,
    /// the other ones are its parents.
    pub fn classes(&self) -> &[String] {
        let a = self.classes.as_ref().unwrap();
        let b = a.as_classes().unwrap();
        b
    }

    /// Returns a class of the object
    pub fn class(&self) -> &str {
        let a = self.classes.as_ref().unwrap();
        &a.as_classes().as_ref().unwrap()[0]
    }

    pub(crate) fn apply_value_refs(&mut self, tree: &[ValueRef]) -> Result<(), Error> {
        self.classes = Some(tree[self.classes_uid as usize].clone());
        if !self.classes.as_ref().unwrap().is_classes() {
            return Err(Error::IncorrectFormat(format!(
                "Incorrent Classes structure (uid: {})",
                self.classes_uid
            )));
        }

        for (key, value) in self.uninit_fields.take().unwrap() {
            match value {
                UninitRefs::RawRefArray(raw_ref_array) => {
                    let mut ref_arr = Vec::with_capacity(raw_ref_array.len());
                    for item in raw_ref_array {
                        if let Some(obj_ref) = tree.get(item as usize) {
                            ref_arr.push(obj_ref.clone())
                        } else {
                            return Err(Error::IncorrectFormat(format!(
                                "Incorrent object uid: {item}"
                            )));
                        }
                    }
                    self.fields.insert(key, ObjectValue::RefArray(ref_arr));
                },
                UninitRefs::RawRef(raw_ref) => {
                    if let Some(obj_ref) = tree.get(raw_ref as usize) {
                        self.fields.insert(key, ObjectValue::Ref(obj_ref.clone()));
                    } else {
                        return Err(Error::IncorrectFormat(format!("Incorrent object uid: {raw_ref}")));
                    }
                },
            }
        }
        Ok(())
    }

    pub(crate) fn from_dict(mut dict: PlistDictionary) -> Result<Self, Error> {
        // unwrapping is safe, we previously check it with is_container()
        let classes_uid = dict.remove("$class").unwrap().into_uid().unwrap().get();
        let mut fields = HashMap::with_capacity(dict.len());
        let mut uninit_fields = HashMap::with_capacity(dict.len());
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
                        return Err(Error::IncorrectFormat(format!(
                            "Array (uid: {classes_uid}) should contain only object references"
                        )));
                    } else {
                        arr_of_uids.push(val.into_uid().unwrap().get());
                    }
                }
                uninit_fields.insert(key, UninitRefs::RawRefArray(arr_of_uids));
                continue;
            } else if obj.as_uid().is_some() {
                uninit_fields.insert(key, UninitRefs::RawRef(obj.into_uid().unwrap().get()));
                continue;
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
            uninit_fields: Some(uninit_fields)
        })
    }
}
