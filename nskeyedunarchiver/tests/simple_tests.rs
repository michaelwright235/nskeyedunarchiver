use std::collections::HashMap;
use std::rc::{Rc, Weak};

use nskeyedunarchiver::{ArchiveValue, DeError, ObjectValue, ValueRef, Data, Decodable, KeyedArchive};

const PLIST_PATH: &str = "./tests_resources/plists/";

fn open_file(name: &str) -> (ValueRef, Vec<Weak<ArchiveValue>>) {
    let archive = KeyedArchive::from_file(format!("{PLIST_PATH}{name}")).unwrap();
    let weak_refs: Vec<Weak<ArchiveValue>> = archive
        .values()
        .iter()
        .map(|v| Rc::downgrade(v))
        .collect();
    (archive.root().unwrap(), weak_refs)
}

// Make sure we don't have dangling references at the end
fn check_rc_strong_count(weak_refs: &[Weak<ArchiveValue>]) {
    for weak in weak_refs {
        let strong_count = weak.strong_count();
        if strong_count != 0 {
            panic!("At the end reference strong count should be 0, found {strong_count}");
        }
    }
}

#[test]
fn plain_string() {
    // -- String: "Some string!"
    let (root, weak_refs) = open_file("plainString.plist");
    let decoded_string = String::decode(&root.into()).unwrap();
    check_rc_strong_count(&weak_refs);
    assert_eq!(decoded_string, "Some string!");
}

#[test]
fn ns_data() {
    let (root, weak_refs) = open_file("nsData.plist");
    let decoded_data = Data::decode(&root.into()).unwrap();
    let ns_data = b"Some data!".to_vec().into();
    assert_eq!(decoded_data, ns_data);
    check_rc_strong_count(&weak_refs);
}

/* SimpleArray */

#[derive(PartialEq, Debug)]
enum SimpleArrayItem {
    String(String),
    Array(Vec<String>),
}

impl Decodable for SimpleArrayItem {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        if let Ok(v) = String::decode(value) {
            return Ok(Self::String(v));
        }
        if let Ok(v) = Vec::<String>::decode(value) {
            return Ok(Self::Array(v));
        }
        Err(DeError::Custom(format!(
            "Undecodable object for enum: {value:?}",
        )))
    }
}

#[test]
fn simple_array() {
    // -- NSArray
    //    -- String: "value1"
    //    -- String: "value2"
    //    -- NSArray
    //       -- String: "innerValue3"
    //       -- String: "innerValue4"

    let (root, weak_refs) = open_file("simpleArray.plist");
    let decoded_data = Vec::<SimpleArrayItem>::decode(&root.into()).unwrap();
    let simple_array = vec![
        SimpleArrayItem::String("value1".into()),
        SimpleArrayItem::String("value2".into()),
        SimpleArrayItem::Array(vec!["innerValue3".into(), "innerValue4".into()]),
    ];
    assert_eq!(decoded_data, simple_array);

    check_rc_strong_count(&weak_refs);
}

#[test]
#[ignore = "Currenty weak references are not supported, so objects with circular references stay in memory."]
fn circular_reference() {
    // Currenty weak references are not supported, so objects with circular references stay in memory.
    // Therefore this test panics

    // -- NSMutableArray   <-|
    //    -- NSMutableArray -^
    let (root, weak_refs) = open_file("circularReference.plist");
    std::mem::drop(root);
    check_rc_strong_count(&weak_refs);
}

/* SimpleDict */

#[derive(PartialEq, Debug)]
enum SimpleDictItem {
    String(String),
    Array(Vec<u8>),
}

impl Decodable for SimpleDictItem {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        if let Ok(v) = String::decode(value) {
            return Ok(Self::String(v));
        }
        if let Ok(v) = Vec::<u8>::decode(value) {
            return Ok(Self::Array(v));
        }
        Err(DeError::Custom(format!(
            "Undecodable object for enum: {value:?}",
        )))
    }
}

#[test]
fn simple_dict() {
    // -- NSDictionary
    //    -- First key  -> String: "First value"
    //    -- Second key -> String: "Second value"
    //    -- Array key  -> NSArray:
    //                      -- Integer: 1
    //                      -- Integer: 2
    //                      -- Integer: 3

    let (root, weak_refs) = open_file("simpleDict.plist");
    let decoded_data = HashMap::<String, SimpleDictItem>::decode(&root.into()).unwrap();
    let simple_dict: HashMap<String, SimpleDictItem> = HashMap::from([
        (
            "First key".into(),
            SimpleDictItem::String("First value".into()),
        ),
        (
            "Second key".into(),
            SimpleDictItem::String("Second value".into()),
        ),
        ("Array key".into(), SimpleDictItem::Array(vec![1, 2, 3])),
    ]);
    assert_eq!(decoded_data, simple_dict);
    check_rc_strong_count(&weak_refs);
}

/* Note */

#[derive(PartialEq, Debug)]
enum NoteArrayMember {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl Decodable for NoteArrayMember {
    fn decode(value: &ObjectValue) -> Result<Self, DeError>
    where
        Self: Sized,
    {
        if let Ok(v) = String::decode(value) {
            return Ok(Self::String(v));
        }
        if let Ok(v) = i64::decode(value) {
            return Ok(Self::Integer(v));
        }
        if let Ok(v) = bool::decode(value) {
            return Ok(Self::Boolean(v));
        }
        Err(DeError::Custom(format!(
            "Undecodable object for enum: {value:?}",
        )))
    }
}

#[derive(PartialEq, Debug)]
struct Note {
    author: String,
    title: String,
    published: bool,
    array: Vec<NoteArrayMember>,
}

impl Decodable for Note {
    fn decode(value: &ObjectValue) -> Result<Self, DeError> {
        let value = {
            let ObjectValue::Ref(value) = value else {
                return Err(DeError::ExpectedObject);
            };
            value.as_object().ok_or(DeError::ExpectedObject)
        }?;
        Ok(Self {
            author: {
                let v = value
                    .as_map()
                    .get("author")
                    .ok_or(DeError::MissingObjectKey(
                        value.class().into(),
                        "author".into(),
                    ))?;
                String::decode(v)?
            },
            title: {
                let v = value
                    .as_map()
                    .get("title")
                    .ok_or(DeError::MissingObjectKey(
                        value.class().into(),
                        "title".into(),
                    ))?;
                String::decode(v)?
            },
            published: {
                if let Some(v) = value.as_map().get("published") {
                    bool::decode(v)?
                } else {
                    Default::default()
                }
            },
            array: {
                let v = value
                    .as_map()
                    .get("array")
                    .ok_or(DeError::MissingObjectKey(
                        value.class().into(),
                        "array".into(),
                    ))?;
                Vec::<NoteArrayMember>::decode(v)?
            },
        })
    }
}

#[test]
fn note() {
    // -- Note
    //    -- author      -> String: "Michael Wright"
    //    -- title       -> String: "Some cool title"
    //    -- published   -> Boolean: true
    //    -- array       -> NSArray:
    //                      -- String: "Hello, World!"
    //                      -- Integer: 42
    //                      -- Boolean: true
    let archive = KeyedArchive::from_file("./tests_resources/plists/note.plist").unwrap();
    let obj = archive.root().unwrap();
    let decoded = Note::decode(&obj.into()).unwrap();

    let note = Note {
        author: "Michael Wright".into(),
        title: "Some cool title".into(),
        published: true,
        array: vec![
            NoteArrayMember::String("Hello, World!".into()),
            NoteArrayMember::Integer(42),
            NoteArrayMember::Boolean(true),
        ],
    };
    assert_eq!(note, decoded);
}
