pub mod de;
mod error;

use enum_as_inner::EnumAsInner;
pub use error::*;
pub use plist::Integer;
use plist::{Dictionary as PlistDictionary, Value as PlistValue};
use std::{collections::HashMap, rc::Rc};

pub(crate) const ARCHIVER: &str = "NSKeyedArchiver";
pub(crate) const ARCHIVER_VERSION: u64 = 100000;

pub(crate) const ARCHIVER_KEY_NAME: &str = "$archiver";
pub(crate) const TOP_KEY_NAME: &str = "$top";
pub(crate) const OBJECTS_KEY_NAME: &str = "$objects";
pub(crate) const VERSION_KEY_NAME: &str = "$version";
pub(crate) const NULL_OBJECT_REFERENCE_NAME: &str = "$null";

pub type ValueRef = Rc<ArchiveValue>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct UniqueId(usize);
impl UniqueId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
    pub fn get(&self) -> usize {
        self.0
    }
}

// Possible values inside of $objects
#[derive(Debug)]
pub(crate) enum ArchiveValueVariant {
    String(String),
    Integer(Integer),
    Real(f64),
    NullRef,
    Classes(Vec<String>),
    Object(Object),
}

macro_rules! as_something {
    ($self:ident, $enum_vname:literal, $typ:ty) => {
        paste::paste! {
            pub fn [<as_ $enum_vname:lower>](&self) -> Option<&$typ> {
                if let ArchiveValueVariant::[<$enum_vname>](v) = &self.value {
                    Some(v)
                } else {
                    None
                }
            }

            pub fn [<is_ $enum_vname:lower>](&self) -> bool {
                if let ArchiveValueVariant::[<$enum_vname>](_) = &self.value {
                    true
                } else {
                    false
                }
            }
        }
    };
}

#[derive(Debug)]
pub struct ArchiveValue {
    value: ArchiveValueVariant,
    unique_id: UniqueId,
}
impl ArchiveValue {
    pub(crate) fn new(value: ArchiveValueVariant, unique_id: UniqueId) -> Self {
        Self { value, unique_id }
    }
    as_something!(self, "String", String);
    as_something!(self, "Integer", Integer);
    as_something!(self, "Real", f64);
    as_something!(self, "Object", Object);
    as_something!(self, "Classes", Vec<String>);

    pub(crate) fn as_object_mut(&mut self) -> Option<&mut Object> {
        if let ArchiveValueVariant::Object(v) = &mut self.value {
            Some(v)
        } else {
            None
        }
    }

    pub fn is_null_ref(&self) -> bool {
        if let ArchiveValueVariant::NullRef = &self.value {
            true
        } else {
            false
        }
    }

    pub fn unique_id(&self) -> &UniqueId {
        &self.unique_id
    }
}

pub struct NSKeyedUnarchiver {
    objects: Vec<ValueRef>,
    top: PlistDictionary,
}

impl NSKeyedUnarchiver {
    pub fn new(plist: PlistValue) -> Result<Self, Error> {
        let Some(mut dict) = plist.into_dictionary() else {
            return Err(IncorrectFormatError::WrongValueType("root", "Dictionary").into());
        };

        // Check $archiver key
        let archiver_key = Self::get_header_key(&mut dict, ARCHIVER_KEY_NAME)?;
        let Some(archiver_str) = archiver_key.as_string() else {
            return Err(IncorrectFormatError::WrongValueType(ARCHIVER_KEY_NAME, "String").into());
        };

        if archiver_str != ARCHIVER {
            return Err(IncorrectFormatError::UnsupportedArchiver.into());
        }

        // Check $version key
        let version_key = Self::get_header_key(&mut dict, VERSION_KEY_NAME)?;
        let Some(version_num) = version_key.as_unsigned_integer() else {
            return Err(IncorrectFormatError::WrongValueType(VERSION_KEY_NAME, "Number").into());
        };

        if version_num != ARCHIVER_VERSION {
            return Err(IncorrectFormatError::UnsupportedArchiverVersion.into());
        }

        // Check $top key
        let top_key = Self::get_header_key(&mut dict, TOP_KEY_NAME)?;
        let Some(top) = top_key.to_owned().into_dictionary() else {
            return Err(IncorrectFormatError::WrongValueType(TOP_KEY_NAME, "Dictionary").into());
        };

        // Check $objects key
        let objects_key = Self::get_header_key(&mut dict, OBJECTS_KEY_NAME)?;
        let Some(raw_objects) = objects_key.into_array() else {
            return Err(IncorrectFormatError::WrongValueType(OBJECTS_KEY_NAME, "Array").into());
        };

        let objects = Self::decode_objects(raw_objects)?;
        Ok(Self { objects, top })
    }

    pub fn top(&self) -> HashMap<String, ValueRef> {
        let mut map = HashMap::with_capacity(self.top.len());
        for (key, value) in &self.top {
            let uid = value.as_uid().unwrap().get() as usize;
            map.insert(key.to_string(), self.objects[uid].clone());
        }
        map
    }

    pub fn values(&self) -> &[ValueRef] {
        &self.objects
    }

    fn get_header_key(dict: &mut PlistDictionary, key: &'static str) -> Result<PlistValue, Error> {
        let Some(objects_value) = dict.remove(key) else {
            return Err(IncorrectFormatError::MissingHeaderKey(key).into());
        };
        Ok(objects_value)
    }

    /// Reads a plist file and creates a new converter for it. It should have a
    /// NSKeyedArchiver plist structure.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error> {
        let val: PlistValue = plist::from_file(path)?;
        Self::new(val)
    }

    /// Reads a plist from a byte slice and creates a new converter for it.
    /// It should have a NSKeyedArchiver plist structure.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let val: PlistValue = plist::from_bytes(bytes)?;
        Self::new(val)
    }

    /// Reads a plist from a seekable byte stream and creates a new converter
    /// for it. It should have a NSKeyedArchiver plist structure.
    pub fn from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<Self, Error> {
        let val: PlistValue = plist::from_reader(reader)?;
        Self::new(val)
    }

    fn is_container(val: &PlistValue) -> bool {
        let Some(dict) = val.as_dictionary() else {
            return false;
        };
        if let Some(cls) = dict.get("$class") {
            cls.as_uid().is_some()
        } else {
            false
        }
    }

    fn decode_objects(objects: Vec<PlistValue>) -> Result<Vec<ValueRef>, Error> {
        let mut decoded_objects = Vec::with_capacity(objects.len());

        for (index, obj) in objects.into_iter().enumerate() {
            let decoded_obj = if let Some(s) = obj.as_string() {
                if s == NULL_OBJECT_REFERENCE_NAME {
                    ArchiveValue::new(ArchiveValueVariant::NullRef, UniqueId::new(index))
                } else {
                    ArchiveValue::new(
                        ArchiveValueVariant::String(obj.into_string().unwrap()),
                        UniqueId::new(index),
                    )
                }
            } else if let PlistValue::Integer(i) = obj {
                ArchiveValue::new(ArchiveValueVariant::Integer(i), UniqueId::new(index))
            } else if let Some(f) = obj.as_real() {
                ArchiveValue::new(ArchiveValueVariant::Real(f), UniqueId::new(index))
            } else if let Some(dict) = obj.as_dictionary() {
                if Self::is_container(&obj) {
                    ArchiveValue::new(
                        ArchiveValueVariant::Object(Object::from_dict(
                            obj.into_dictionary().unwrap(),
                        )?),
                        UniqueId::new(index),
                    )
                } else if dict.contains_key("$classes") {
                    if let Some(classes_arr) = obj
                        .into_dictionary()
                        .unwrap()
                        .remove("$classes")
                        .unwrap()
                        .into_array()
                    {
                        let mut classes = Vec::with_capacity(classes_arr.len());
                        for class in classes_arr {
                            if let Some(s) = class.into_string() {
                                classes.push(s)
                            } else {
                                return Err(Error::DecodingObjectError(
                                    "Incorrect Classes object".to_string(),
                                ));
                            }
                        }
                        ArchiveValue::new(
                            ArchiveValueVariant::Classes(classes),
                            UniqueId::new(index),
                        )
                    } else {
                        return Err(Error::DecodingObjectError(
                            "Incorrect Classes object".to_string(),
                        ));
                    }
                } else {
                    return Err(Error::DecodingObjectError(
                        "Unexpected object type".to_string(),
                    ));
                }
            } else {
                return Err(Error::DecodingObjectError(
                    "Unexpected object type".to_string(),
                ));
            };
            decoded_objects.push(Rc::new(decoded_obj));
        }

        // In order to avoid using RefCell to write object references into
        // them only once, we can use this hack
        let mut decoded_objects_raw = Vec::with_capacity(decoded_objects.len());
        for object in &decoded_objects {
            let raw = Rc::into_raw(Rc::clone(object)) as *mut ArchiveValue;
            decoded_objects_raw.push(raw);
            unsafe { Rc::decrement_strong_count(raw) };
        }

        for ptr in &decoded_objects_raw {
            let a = unsafe { &mut **ptr };
            if let Some(obj) = a.as_object_mut() {
                obj.apply_value_refs(&decoded_objects)?
            }
        }
        Ok(decoded_objects)
    }
}

macro_rules! get_key {
    ($self:ident, $key:ident, $typ:literal) => {{
        if !$self.contains_key($key) {
            return Err(DeError::MissingObjectKey(format!(
                "Missing key '{0}' for object '{1}'",
                $key,
                $self.class()
            )));
        }
        let raw_object = $self.fields.get($key).unwrap();
        let obj = paste::paste! {raw_object.[<as_$typ>]() };
        if obj.is_none() {
            return Err(DeError::IncorrectObjectValueType(format!(
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

#[derive(Debug, EnumAsInner, Clone)]
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

    pub fn decode_real(&self, key: &str) -> Result<f64, DeError> {
        Ok(*get_key!(self, key, "real"))
    }

    pub fn decode_integer(&self, key: &str) -> Result<Integer, DeError> {
        Ok(*get_key!(self, key, "integer"))
    }

    pub fn decode_string(&self, key: &str) -> Result<String, DeError> {
        // As far as I can tell all strings inside of objects are
        // linked with UIDs
        let obj = get_key!(self, key, "ref");
        let Some(string) = obj.as_string() else {
            return Err(DeError::IncorrectObjectValueType(format!(
                "Incorrect value type of '{0}' for object '{1}'. Expected '{2}' for key '{3}'",
                "object",
                self.class(),
                "string",
                key
            )));
        };

        Ok(string.to_string())
    }

    pub fn decode_object(&self, key: &str) -> Result<ValueRef, DeError> {
        let obj = get_key!(self, key, "ref");
        Ok(obj.clone())
    }

    pub fn decode_array(&self, key: &str) -> Result<Vec<ValueRef>, DeError> {
        let array = get_key!(self, key, "ref_array");
        let mut refs = Vec::with_capacity(array.len());
        for item in array {
            refs.push(item.clone());
        }
        Ok(refs)
    }

    pub fn is_null_ref(&self, key: &str) -> Result<bool, DeError> {
        if !self.contains_key(key) {
            return Err(DeError::MissingObjectKey(format!(
                "Missing key '{0}' for object '{1}'",
                self.class(),
                key
            )));
        }
        Ok(self.fields.get(key).unwrap().is_null_ref())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    pub fn classes(&self) -> &[String] {
        let a = self.classes.as_ref().unwrap();
        let b = a.as_classes().unwrap();
        b
    }

    pub fn class(&self) -> &String {
        let a = self.classes.as_ref().unwrap();
        &a.as_classes().as_ref().unwrap()[0]
    }

    pub(crate) fn apply_value_refs(&mut self, tree: &[ValueRef]) -> Result<(), Error> {
        self.classes = Some(tree[self.classes_uid as usize].clone());
        if !self.classes.as_ref().unwrap().is_classes() {
            return Err(Error::DecodingObjectError(
                "Incorrent Classes structure".to_string(),
            ));
        }

        for value in self.fields.values_mut() {
            if let ObjectValue::RawRef(r) = value {
                *value = ObjectValue::Ref(tree[*r as usize].clone());
            }
            if let ObjectValue::RawRefArray(arr) = value {
                let mut ref_arr = Vec::with_capacity(arr.len());
                for item in arr {
                    ref_arr.push(tree[*item as usize].clone())
                }
                *value = ObjectValue::RefArray(ref_arr);
            }
        }
        Ok(())
    }

    pub(crate) fn from_dict(mut dict: PlistDictionary) -> Result<Self, Error> {
        let classes_uid = dict.remove("$class").unwrap().into_uid().unwrap().get(); // unwrapping is safe, we previously check it with is_container()
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
                        return Err(Error::DecodingObjectError(
                            "Array should countain only object references".to_string(),
                        ));
                    } else {
                        arr_of_uids.push(val.into_uid().unwrap().get());
                    }
                }
                ObjectValue::RawRefArray(arr_of_uids)
            } else if obj.as_uid().is_some() {
                ObjectValue::RawRef(obj.into_uid().unwrap().get())
            } else {
                return Err(Error::DecodingObjectError(format!(
                    "Enexpected object value type: {:?}",
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
