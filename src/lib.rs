mod error;

use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, OnceCell, Ref, RefCell}, collections::HashMap, rc::{Rc, Weak}};

use log::debug;
use paste::paste;
pub use plist;
pub use error::*;
use plist::{Dictionary, Integer, Value};

pub(crate) const ARCHIVER: &str = "NSKeyedArchiver";
pub(crate) const ARCHIVER_VERSION: u64 = 100000;

pub(crate) const ARCHIVER_KEY_NAME: &str = "$archiver";
pub(crate) const TOP_KEY_NAME: &str = "$top";
pub(crate) const OBJECTS_KEY_NAME: &str = "$objects";
pub(crate) const VERSION_KEY_NAME: &str = "$version";
pub(crate) const NULL_OBJECT_REFERENCE_NAME: &str = "$null";

macro_rules! get_key {
    ($self:ident, $key:ident, $typ:literal) => {
        paste! {
            {
                if !$self.contains_key($key) {
                    return Err(Error::MissingObjectKey($self.class().to_string(), $key.to_string()))
                }
                let obj = $self.fields.get($key).unwrap().[<as_$typ>]();
                if obj.is_none() {
                    return Err(Error::WrongObjectValueType($typ.to_string(), $key.to_string()))
                }
                obj.unwrap()
            }
        }
    };
}

// Possible values inside of $objects
#[derive(Debug)]
enum ArchiveValue {
    String(String),
    Integer(Integer),
    F64(f64),
    NullRef,
    Classes(Vec<String>),
    Object(Object),
}

impl ArchiveValue {
    pub fn as_object(&self) -> Option<&Object> {
        if let Self::Object(o) = &self {
            Some(o)
        } else {
            None
        }
    }
    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        if let Self::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }
    pub fn as_classes(&self) -> Option<&[String]> {
        if let Self::Classes(o) = &self {
            Some(o)
        } else {
            None
        }
    }
}

pub struct NSKeyedUnarchiver {
    objects: Vec<Rc<RefCell<ArchiveValue>>>,
    top: Dictionary,
}

impl NSKeyedUnarchiver {
    pub fn new(plist: Value) -> Result<Self, Error> {
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

        let objects = Self::decode_objects(raw_objects);
        Ok(Self {
            objects,
            top
        })
    }

    pub fn test(&self) -> Ref<'_, ArchiveValue> {
        println!("{:#?}", self.objects[2]);
        for obj in &self.objects {
            let a = obj.as_ref();
        }
        let a = &self.objects[0];
        let b = a.as_ref().borrow();
        b
    }

    fn get_header_key(dict: &mut Dictionary, key: &'static str) -> Result<Value, Error> {
        let Some(objects_value) = dict.remove(key) else {
            return Err(IncorrectFormatError::MissingHeaderKey(key).into());
        };
        Ok(objects_value)
    }

    /// Reads a plist file and creates a new converter for it. It should have a
    /// NSKeyedArchiver plist structure.
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error> {
        let val: Value = plist::from_file(path)?;
        Self::new(val)
    }

    /// Reads a plist from a byte slice and creates a new converter for it.
    /// It should have a NSKeyedArchiver plist structure.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let val: Value = plist::from_bytes(bytes)?;
        Self::new(val)
    }

    /// Reads a plist from a seekable byte stream and creates a new converter
    /// for it. It should have a NSKeyedArchiver plist structure.
    pub fn from_reader<R: std::io::Read + std::io::Seek>(
        reader: R,
    ) -> Result<Self, Error> {
        let val: Value = plist::from_reader(reader)?;
        Self::new(val)
    }

    fn is_container(val: &Value) -> bool {
        let Some(dict) = val.as_dictionary() else {
            return false;
        };
        if let Some(cls) = dict.get("$class") {
            cls.as_uid().is_some()
        } else {
            false
        }
    }

    fn decode_objects(objects: Vec<Value>) -> Vec<Rc<RefCell<ArchiveValue>>> {
        let mut decoded_objects = Vec::with_capacity(objects.len());
        for obj in objects {
            let decoded_obj =
            if let Some(s) = obj.as_string() {
                if s == NULL_OBJECT_REFERENCE_NAME {
                    ArchiveValue::NullRef
                } else {
                    ArchiveValue::String(obj.into_string().unwrap())
                }
            } else if let Value::Integer(i) = obj {
                ArchiveValue::Integer(i)
            } else if let Some(f) = obj.as_real() {
                ArchiveValue::F64(f)
            } else if let Some(dict) = obj.as_dictionary() {
                if Self::is_container(&obj) {
                    ArchiveValue::Object(Object::from_dict(obj.into_dictionary().unwrap()))
                }
                else if dict.contains_key("$classes") {
                    if let Some(classes_arr) = obj.into_dictionary().unwrap().remove("$classes").unwrap().into_array() {
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
            decoded_objects.push(Rc::new(RefCell::new(decoded_obj)));
        }

        for object in &decoded_objects {
            let mut a = object.as_ref().borrow_mut();
            if let Some(obj) = a.as_object_mut() {
                obj.apply_object_tree(&decoded_objects)
            }
        }
        decoded_objects
    }
}

#[derive(Debug)]
enum ObjectValue {
    String(String),
    Integer(Integer),
    F64(f64),
    Boolean(bool),
    Data(Vec<u8>),
    RefArray(Vec<Rc<RefCell<ArchiveValue>>>),
    Ref(Rc<RefCell<ArchiveValue>>),
    NullRef,

    // Don't use them
    RawRefArray(Vec<u64>), // vector of uids
    RawRef(u64), // uid
}

#[derive(Debug)]
pub struct Object {
    classes: Option<Rc<RefCell<ArchiveValue>>>,
    classes_uid: u64,
    fields: HashMap<String, ObjectValue>,
}

impl Object {
    pub fn decode_bool(&self, key: &str) -> Result<bool, Error> {
        //Ok(get_key!(self, key, "boolean"))
        todo!()
    }

    pub fn decode_data(&self, key: &str) -> Result<Vec<u8>, Error> {
        //Ok(get_key!(self, key, "data").to_vec())
        todo!()
    }

    pub fn decode_f64(&self, key: &str) -> Result<f64, Error> {
        //Ok(get_key!(self, key, "real"))
        todo!()
    }

    pub fn decode_i64(&self, key: &str) -> Result<i64, Error> {
        //Ok(get_key!(self, key, "signed_integer"))
        todo!()
    }

    pub fn decode_u64(&self, key: &str) -> Result<u64, Error> {
        //Ok(get_key!(self, key, "unsigned_integer"))
        todo!()
    }

    pub fn decode_string(&self, key: &str) -> Result<String, Error> {
        // As far as I can tell all strings inside of objects are
        // linked with UIDs
        // let uid = get_key!(self, key, "uid").get();
        // let Some(obj) = self.objects_tree.get(uid as usize) else {
        //     return Err(Error::MissingObject(uid));
        // };
        // if let Some(s) = obj.as_string() {
        //     return Ok(s.to_string())
        // } else {
        //     return Err(Error::WrongObjectValueType("string".to_string(), key.to_string()))
        // }
        todo!()
    }

    pub fn decode_object(&self, key: &str) -> Result<(), Error> { // -> Result<&Object, Error>
        // let uid = get_key!(self, key, "uid").get();
        // let Some(obj) = self.objects_tree.get(uid as usize) else {
        //     return Err(Error::MissingObject(uid));
        // };
        // if !Self::is_container(obj) {
        //     return Err(Error::WrongObjectValueType("dictionary".to_string(), key.to_string()));
        // }
        // return new object from dict
        todo!()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    pub fn classes(&self) -> Vec<String> {
        let a = self.classes.as_ref().unwrap().as_ref().borrow();
        let b = a.as_classes().unwrap();
        b.to_vec()
    }

    pub fn class(&self) -> String {
        let a = self.classes.as_ref().unwrap().as_ref().borrow();
        a.as_classes().unwrap()[0].to_string()
    }

    pub(crate) fn apply_object_tree(&mut self, tree: &[Rc<RefCell<ArchiveValue>>]) {
        self.classes = Some(tree[self.classes_uid as usize].clone());

        for (_, value) in &mut self.fields {
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

    pub(crate) fn from_dict(mut dict: Dictionary) -> Self {
        let classes_uid = dict.remove("$class").unwrap().into_uid().unwrap().get(); // unwrapping is safe, we previously check it with is_container()
        let mut fields = HashMap::with_capacity(dict.len());
        for (key, obj) in dict {
            let decoded_obj =
            if let Some(s) = obj.as_string() {
                if s == NULL_OBJECT_REFERENCE_NAME {
                    ObjectValue::NullRef
                } else {
                    ObjectValue::String(obj.into_string().unwrap())
                }
            } else if let Value::Integer(i) = obj {
                ObjectValue::Integer(i)
            } else if let Some(f) = obj.as_real() {
                ObjectValue::F64(f)
            } else if let Some(b) = obj.as_boolean() {
                ObjectValue::Boolean(b)
            } else if obj.as_data().is_some() {
                ObjectValue::Data(obj.into_data().unwrap())
            } else if let Some(arr) = obj.as_array() {
                let mut arr_of_uids = Vec::with_capacity(arr.len());
                for val in obj.into_array().unwrap() {
                    if val.as_uid().is_none() {
                        panic!("Array should countain only uids")
                    } else {
                        arr_of_uids.push(val.into_uid().unwrap().get());
                    }
                }
                ObjectValue::RawRefArray(arr_of_uids)
            } else if obj.as_uid().is_some() {
                ObjectValue::RawRef(obj.into_uid().unwrap().get())
            } else {
                panic!("Unexpected object value type: {:?}", obj)
            };
            fields.insert(key, decoded_obj);
        }
        Self {
            classes: None,
            classes_uid,
            fields,
        }
    }
}

pub trait Decodable {
    fn decode(object: Object) -> Self;
}

#[cfg(test)]
mod tests {
    use simplelog::{SimpleLogger, LevelFilter, Config};
    use crate::NSKeyedUnarchiver;

    #[test]
    fn test1() {
        let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());
        let unarchiver = NSKeyedUnarchiver::from_file("./MainWindow.nib").unwrap();
        unarchiver.test();
    }
}
