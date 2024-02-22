pub mod de;
mod error;
mod object;

pub use error::*;
use object::*;
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
