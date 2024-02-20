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

pub type ObjectRef = Rc<ArchiveValue>;
// Possible values inside of $objects
#[derive(Debug, EnumAsInner)]
pub enum ArchiveValue {
    String(String),
    Integer(Integer),
    Real(f64),
    NullRef,
    Classes(Vec<String>),
    Object(Object),
}

pub struct NSKeyedUnarchiver {
    objects: Vec<ObjectRef>,
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

    pub fn top(&self) -> HashMap<String, ObjectRef> {
        let mut map = HashMap::with_capacity(self.top.len());
        for (key, value) in &self.top {
            let uid = value.as_uid().unwrap().get() as usize;
            map.insert(key.to_string(), self.objects[uid].clone());
        }
        map
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

    fn decode_objects(objects: Vec<PlistValue>) -> Result<Vec<ObjectRef>, Error> {
        let mut decoded_objects = Vec::with_capacity(objects.len());
        for obj in objects {
            let decoded_obj = if let Some(s) = obj.as_string() {
                if s == NULL_OBJECT_REFERENCE_NAME {
                    ArchiveValue::NullRef
                } else {
                    ArchiveValue::String(obj.into_string().unwrap())
                }
            } else if let PlistValue::Integer(i) = obj {
                ArchiveValue::Integer(i)
            } else if let Some(f) = obj.as_real() {
                ArchiveValue::Real(f)
            } else if let Some(dict) = obj.as_dictionary() {
                if Self::is_container(&obj) {
                    ArchiveValue::Object(Object::from_dict(obj.into_dictionary().unwrap())?)
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
                                panic!("Incorrect Classes object")
                            }
                        }
                        ArchiveValue::Classes(classes)
                    } else {
                        panic!("Incorrect Classes object")
                    }
                } else {
                    panic!("Unexpected object type")
                }
            } else {
                panic!("Unexpected object type")
            };
            decoded_objects.push(Rc::new(decoded_obj));
        }

        // In order to avoid using RefCell to write object references into
        // them only once, we can use this hack
        let mut decoded_objects_raw = Vec::with_capacity(decoded_objects.len());
        for object in &decoded_objects {
            let raw = Rc::into_raw(Rc::clone(object)) as *mut ArchiveValue;
            decoded_objects_raw.push(raw.clone());
            unsafe { Rc::decrement_strong_count(raw) };
        }

        for ptr in &decoded_objects_raw {
            let a = unsafe { &mut **ptr };
            if let Some(obj) = a.as_object_mut() {
                obj.apply_object_tree(&decoded_objects)
            }
        }
        Ok(decoded_objects)
    }
}

macro_rules! get_key {
    ($self:ident, $key:ident, $typ:literal) => {{
        if !$self.contains_key($key) {
            return Err(
                DeError::MissingObjectKey(
                    format!(
                        "Missing key '{0}' for object '{1}'",
                        $key,
                        $self.class()
                    )
                )
            );
        }
        let raw_object = $self.fields.get($key).unwrap();
        let obj = paste::paste! {raw_object.[<as_$typ>]() };
        if obj.is_none() {
            return Err(
                DeError::IncorrectObjectValueType(
                    format!(
                        "Incorrect value type of '{0}' for object '{1}'. Expected '{2}' for key '{3}'",
                        $typ,
                        $self.class(),
                        raw_object.as_plain_type(),
                        $key.to_string()
                    )
                )
            );
        }
        obj.unwrap()
    }}
}

#[derive(Debug, EnumAsInner, Clone)]
enum ObjectValue {
    String(String),
    Integer(Integer),
    Real(f64),
    Boolean(bool),
    Data(Vec<u8>),
    RefArray(Vec<ObjectRef>),
    Ref(ObjectRef),
    NullRef,

    // Don't use them
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
    classes: Option<ObjectRef>,
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

    pub fn decode_object(&self, key: &str) -> Result<ObjectRef, DeError> {
        // -> Result<&Object, Error>
        let obj = get_key!(self, key, "ref");
        Ok(obj.clone())
    }

    pub fn decode_array(&self, key: &str) -> Result<Vec<ObjectRef>, DeError> {
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

    pub fn classes(&self) -> Vec<String> {
        let a = self.classes.as_ref().unwrap();
        let b = a.as_classes().unwrap();
        b.to_vec()
    }

    pub fn class(&self) -> String {
        let a = self.classes.as_ref().unwrap();
        a.as_classes().unwrap()[0].to_string()
    }

    pub(crate) fn apply_object_tree(&mut self, tree: &[ObjectRef]) {
        self.classes = Some(tree[self.classes_uid as usize].clone());
        if !self.classes.as_ref().unwrap().is_classes() {
            panic!("Incorrent Classes structure")
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

#[cfg(test)]
mod tests {
    use crate::NSKeyedUnarchiver;
    use simplelog::{Config, LevelFilter, SimpleLogger};

    #[test]
    fn test1() {
        let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());
        let unarchiver = NSKeyedUnarchiver::from_file("./MainWindow.nib").unwrap();
        println!("{:#?}", unarchiver.top());
    }

    #[test]
    fn test2() {
        let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());
        let unarchiver = NSKeyedUnarchiver::from_file("./NSAffineTransform2.plist").unwrap();
        println!("{:#?}", unarchiver.top());
    }
}
