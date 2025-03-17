pub mod de;
mod error;
mod object;

pub use error::*;
pub use object::*;
pub use plist::Integer;
use plist::{Dictionary as PlistDictionary, Value as PlistValue};
use std::{collections::HashMap, rc::Rc};

#[cfg(feature = "derive")]
pub use nskeyedunarchiver_derive::Decodable;

const ARCHIVER: &str = "NSKeyedArchiver";
const ARCHIVER_VERSION: u64 = 100000;

const ARCHIVER_KEY_NAME: &str = "$archiver";
const TOP_KEY_NAME: &str = "$top";
const OBJECTS_KEY_NAME: &str = "$objects";
const VERSION_KEY_NAME: &str = "$version";
const NULL_OBJECT_REFERENCE_NAME: &str = "$null";

/// An [Rc] smart pointer to an [ArchiveValue]
pub type ValueRef = Rc<ArchiveValue>;

/// A unique id of an archive value.
///
/// When decoding complex structures this it may help with indentifying repeatable
/// values.
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

/// Possible values inside of $objects
#[derive(Debug, PartialEq)]
pub(crate) enum ArchiveValueVariant {
    Boolean(bool),
    Classes(Vec<String>),
    Data(Vec<u8>),
    Integer(Integer),
    NullRef,
    Object(Object),
    Real(f64),
    String(String),
}

/// Represents a single value contained inside of an archive.
///
/// The possible values are: [String], [Integer], [f64], Vec<u8>, [bool],
/// `NullRef` (a `$null` reference ), `Classes` (an array of class strings), [Object].
#[derive(Debug, PartialEq)]
pub struct ArchiveValue {
    value: ArchiveValueVariant,
    unique_id: UniqueId,
}
impl ArchiveValue {
    pub(crate) fn new(value: ArchiveValueVariant, unique_id: UniqueId) -> Self {
        Self { value, unique_id }
    }

    /// Checks if a contained value is a [bool].
    pub fn as_boolean(&self) -> Option<bool> {
        if let ArchiveValueVariant::Boolean(b) = &self.value {
            Some(*b)
        } else {
            None
        }
    }

    /// Checks if a contained value is a [bool].
    pub fn is_boolean(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::Boolean(_))
    }

    /// Returns [Some] with a reference to a contained [String] if a value represents it or [None] if it doesn't.
    pub fn as_string(&self) -> Option<&str> {
        if let ArchiveValueVariant::String(v) = &self.value {
            Some(v.as_str())
        } else {
            None
        }
    }

    /// Checks if a contained value is a [String].
    pub fn is_string(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::String(_))
    }

    /// Returns [Some] with a reference to a contained [Integer] if a value represents it or [None] if it doesn't.
    pub fn as_integer(&self) -> Option<&Integer> {
        if let ArchiveValueVariant::Integer(v) = &self.value {
            Some(v)
        } else {
            None
        }
    }

    /// Checks if a contained value is an [Integer].
    pub fn is_integer(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::Integer(_))
    }

    /// Returns [Some] with a reference to a contained float ([f64]) if a value represents it or [None] if it doesn't.
    pub fn as_float(&self) -> Option<f64> {
        if let ArchiveValueVariant::Real(v) = &self.value {
            Some(*v)
        } else {
            None
        }
    }

    /// Checks if a contained value is a float ([f64]).
    pub fn is_float(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::Real(_))
    }

    /// Returns [Some] with a reference to a contained [Integer] if a value represents it or [None] if it doesn't.
    pub fn as_data(&self) -> Option<&[u8]> {
        if let ArchiveValueVariant::Data(v) = &self.value {
            Some(v)
        } else {
            None
        }
    }

    /// Checks if a contained value is an [Integer].
    pub fn is_data(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::Data(_))
    }

    /// Returns [Some] with a reference to a contained [Object] if a value represents it or [None] if it doesn't.
    pub fn as_object(&self) -> Option<&Object> {
        if let ArchiveValueVariant::Object(v) = &self.value {
            Some(v)
        } else {
            None
        }
    }

    pub(crate) fn as_object_mut(&mut self) -> Option<&mut Object> {
        if let ArchiveValueVariant::Object(v) = &mut self.value {
            Some(v)
        } else {
            None
        }
    }

    /// Checks if a contained value is an [Object].
    pub fn is_object(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::Object(_))
    }

    /// Returns [Some] with a slice of class strings if a value represents it or [None] if it doesn't.
    pub fn as_classes(&self) -> Option<&[String]> {
        if let ArchiveValueVariant::Classes(v) = &self.value {
            Some(v)
        } else {
            None
        }
    }

    /// Checks if a contained value is class strings.
    pub fn is_classes(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::Classes(_))
    }

    /// Checks if a contained value is a null reference.
    pub fn is_null_ref(&self) -> bool {
        matches!(&self.value, ArchiveValueVariant::NullRef)
    }

    /// Returns a [UniqueId] of a given value.
    pub fn unique_id(&self) -> &UniqueId {
        &self.unique_id
    }
}

pub struct NSKeyedUnarchiver {
    objects: Vec<ValueRef>,
    top: PlistDictionary,
}

impl NSKeyedUnarchiver {
    /// Creates a new unarchiver from a [plist::Value]. It should be the root
    /// value of a plist and have a NSKeyedArchiver plist structure.
    ///
    /// Returns an instance of itself or an [Error] if something went wrong.
    pub fn new(plist: PlistValue) -> Result<Self, Error> {
        let Some(mut dict) = plist.into_dictionary() else {
            return Err(Error::IncorrectFormat(
                "Expected root key to be a type of 'Dictionary'".into(),
            ));
        };

        // Check $archiver key
        let archiver_key = Self::get_header_key(&mut dict, ARCHIVER_KEY_NAME)?;
        let Some(archiver_str) = archiver_key.as_string() else {
            return Err(Error::IncorrectFormat(format!(
                "Expected '{ARCHIVER_KEY_NAME}' key to be a type of 'String'"
            )));
        };

        if archiver_str != ARCHIVER {
            return Err(Error::IncorrectFormat(format!(
                "Unsupported archiver. Only '{ARCHIVER}' is supported"
            )));
        }

        // Check $version key
        let version_key = Self::get_header_key(&mut dict, VERSION_KEY_NAME)?;
        let Some(version_num) = version_key.as_unsigned_integer() else {
            return Err(Error::IncorrectFormat(format!(
                "Expected '{VERSION_KEY_NAME}' key to be a type of 'Number'"
            )));
        };

        if version_num != ARCHIVER_VERSION {
            return Err(Error::IncorrectFormat(format!(
                "Unsupported archiver version. Only '{ARCHIVER_VERSION}' is supported"
            )));
        }

        // Check $top key
        let top_key = Self::get_header_key(&mut dict, TOP_KEY_NAME)?;
        let Some(top) = top_key.to_owned().into_dictionary() else {
            return Err(Error::IncorrectFormat(format!(
                "Expected '{TOP_KEY_NAME}' key to be a type of 'Dictionary'"
            )));
        };

        // Check $objects key
        let objects_key = Self::get_header_key(&mut dict, OBJECTS_KEY_NAME)?;
        let Some(raw_objects) = objects_key.into_array() else {
            return Err(Error::IncorrectFormat(format!(
                "Expected '{OBJECTS_KEY_NAME}' key to be a type of 'Array'"
            )));
        };

        let objects = Self::decode_objects(raw_objects)?;
        Ok(Self { objects, top })
    }

    /// Returns a [HashMap] created from the `$top` value.
    ///
    /// If there's only one value inside of `$top`, use `get("root")` to get it.
    pub fn top(&self) -> HashMap<String, ValueRef> {
        let mut map = HashMap::with_capacity(self.top.len());
        for (key, value) in &self.top {
            if let Some(uid) = value.as_uid() {
                map.insert(key.to_string(), self.objects[uid.get() as usize].clone());
            }
        }
        map
    }

    /// Returns all values contained inside of an archive. One may rarely use this.
    pub fn values(&self) -> &[ValueRef] {
        &self.objects
    }

    /// Gets a key from a [plist::Dictionary] or an [Error] if it doesn't exist.
    fn get_header_key(dict: &mut PlistDictionary, key: &'static str) -> Result<PlistValue, Error> {
        let Some(objects_value) = dict.remove(key) else {
            return Err(Error::IncorrectFormat(format!(
                "Missing '{key}' header key"
            )));
        };
        Ok(objects_value)
    }

    /// Reads a plist file and creates a new unarchiver from it.
    /// It should have a NSKeyedArchiver plist structure.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error> {
        let val: PlistValue = plist::from_file(path)?;
        Self::new(val)
    }

    /// Reads a plist from a byte slice and creates a new unarchiver from it.
    /// It should have a NSKeyedArchiver plist structure.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let val: PlistValue = plist::from_bytes(bytes)?;
        Self::new(val)
    }

    /// Reads a plist from a seekable byte stream and creates a new unarchiver from it.
    /// It should have a NSKeyedArchiver plist structure.
    pub fn from_reader<R: std::io::Read + std::io::Seek>(reader: R) -> Result<Self, Error> {
        let val: PlistValue = plist::from_reader(reader)?;
        Self::new(val)
    }

    /// Checks if a [plist::Value] has an object structure.
    fn is_container(val: &PlistDictionary) -> bool {
        if let Some(cls) = val.get("$class") {
            cls.as_uid().is_some()
        } else {
            false
        }
    }

    /// Decodes all values into a vector of Rc<[ArchiveValue]>. Returns an [Error]
    /// if something went wrong.
    fn decode_objects(objects: Vec<PlistValue>) -> Result<Vec<ValueRef>, Error> {
        let mut decoded_objects = Vec::with_capacity(objects.len());

        for (index, obj) in objects.into_iter().enumerate() {
            let decoded_obj = match obj {
                PlistValue::Dictionary(mut dict) => {
                    if Self::is_container(&dict) {
                        ArchiveValue::new(
                            ArchiveValueVariant::Object(Object::from_dict(dict)?),
                            UniqueId::new(index),
                        )
                    } else if dict.contains_key("$classes") {
                        match dict.remove("$classes").unwrap().into_array() {
                            Some(classes_arr) => {
                                let mut classes = Vec::with_capacity(classes_arr.len());
                                for class in classes_arr {
                                    match class.into_string() {
                                        Some(s) => classes.push(s),
                                        _ => {
                                            return Err(Error::IncorrectFormat(
                                                "Incorrect Classes object".into(),
                                            ));
                                        }
                                    }
                                }
                                ArchiveValue::new(
                                    ArchiveValueVariant::Classes(classes),
                                    UniqueId::new(index),
                                )
                            }
                            _ => {
                                return Err(Error::IncorrectFormat(
                                    "Incorrect Classes object".into(),
                                ));
                            }
                        }
                    } else {
                        return Err(Error::IncorrectFormat("Unexpected object type".into()));
                    }
                }
                PlistValue::Boolean(b) => {
                    ArchiveValue::new(ArchiveValueVariant::Boolean(b), UniqueId::new(index))
                }
                PlistValue::Data(data) => ArchiveValue::new(
                    ArchiveValueVariant::Data(data.to_vec()),
                    UniqueId::new(index),
                ),
                PlistValue::Real(real) => {
                    ArchiveValue::new(ArchiveValueVariant::Real(real), UniqueId::new(index))
                }
                PlistValue::Integer(integer) => {
                    ArchiveValue::new(ArchiveValueVariant::Integer(integer), UniqueId::new(index))
                }
                PlistValue::String(string) => {
                    if string == NULL_OBJECT_REFERENCE_NAME {
                        ArchiveValue::new(ArchiveValueVariant::NullRef, UniqueId::new(index))
                    } else {
                        ArchiveValue::new(ArchiveValueVariant::String(string), UniqueId::new(index))
                    }
                }
                _ => {
                    return Err(Error::IncorrectFormat(format!(
                        "Unexpected object type: {obj:?}"
                    )));
                }
            };
            decoded_objects.push(Rc::new(decoded_obj));
        }

        // In order to avoid using RefCell to write object references into
        // them only once, we can use this hack
        let mut decoded_objects_raw = Vec::with_capacity(decoded_objects.len());
        for object in &decoded_objects {
            let raw = Rc::as_ptr(object) as *mut ArchiveValue;
            decoded_objects_raw.push(raw);
        }

        for ptr in &decoded_objects_raw {
            // it's safe, all objects are still in memory
            let a = unsafe { &mut **ptr };
            if let Some(obj) = a.as_object_mut() {
                obj.apply_value_refs(&decoded_objects)?
            }
        }
        Ok(decoded_objects)
    }
}
