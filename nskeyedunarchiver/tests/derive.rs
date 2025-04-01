#![cfg(feature = "derive")]

use nskeyedunarchiver::{Data, Decodable, KeyedArchive, ObjectValue, derive::Decodable};
use std::collections::HashMap;

#[derive(Decodable, Debug, PartialEq)]
struct NSAffineTransform {
    #[decodable(rename = "NSTransformStruct")]
    nstransform_struct: Option<Data>,
}

#[derive(Decodable, Debug, PartialEq)]
struct NSMutableAttributedString {
    #[decodable(rename = "NSAttributeInfo")]
    nsattribute_info: Data,
    #[decodable(rename = "NSAttributes")]
    nsattributes: Vec<HashMap<String, NSColor>>,
    #[decodable(rename = "NSString")]
    nsstring: String,
}

#[derive(Decodable, Debug, PartialEq)]
struct NSColor {
    #[decodable(rename = "NSColorSpace")]
    nscolor_space: i64,
    #[decodable(rename = "NSComponents")]
    nscomponents: Data,
    #[decodable(rename = "NSRGB")]
    nsrgb: Data,
    #[decodable(rename = "NSCustomColorSpace")]
    nscustom_color_space: Foo,
    #[decodable(skip)] // Default::default()
    my_field: String,
}

#[derive(Decodable, Debug, PartialEq)]
#[decodable(rename = "NSColorSpace")]
struct Foo {
    // it's too big to put in this test
    //#[decodable(rename = "NSICC")]
    //nsicc: Data,
    #[decodable(rename = "NSID")]
    nsid: i64,
}

#[derive(Decodable, Debug, PartialEq)]
enum ArrayMember {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Decodable, Debug, PartialEq)]
struct Note {
    author: String,
    title: String,
    #[decodable(default)]
    published: bool,
    array: Vec<ArrayMember>,
    not_present: Option<String>,
    #[decodable(unhandled)]
    unhandled: HashMap<String, ObjectValue>,
}

#[test]
fn note() {
    let unarchiver = KeyedArchive::from_file("./tests_resources/plists/note.plist").unwrap();
    let obj = unarchiver.root().unwrap();
    let decoded = Note::decode(&obj.into()).unwrap();

    let note = Note {
        author: "Michael Wright".into(),
        title: "Some cool title".into(),
        published: true,
        array: vec![
            ArrayMember::String("Hello, World!".into()),
            ArrayMember::Integer(42),
            ArrayMember::Boolean(true),
        ],
        not_present: None,
        unhandled: HashMap::new(),
    };
    assert_eq!(note, decoded);
}

#[derive(Decodable, Debug, PartialEq)]
enum DictMember {
    String(String),
    IntArray(Vec<i64>),
}

#[test]
fn nsaffine_transform() {
    let unarchiver =
        KeyedArchive::from_file("./tests_resources/plists/NSAffineTransform.plist").unwrap();
    let obj = unarchiver.root().unwrap();
    let decoded = NSAffineTransform::decode(&obj.into()).unwrap();
    let eq = NSAffineTransform {
        nstransform_struct: Some(
            vec![
                63, 118, 176, 124, 62, 136, 211, 120, 190, 136, 211, 120, 63, 118, 176, 124, 0, 0,
                0, 0, 0, 0, 0, 0,
            ]
            .into(),
        ),
    };
    assert_eq!(decoded, eq);
}

#[test]
fn nsmutable_attributed_string() {
    let unarchiver =
        KeyedArchive::from_file("./tests_resources/plists/NSMutableAttributedString.plist")
            .unwrap();
    let obj = unarchiver.root().unwrap();
    let decoded = NSMutableAttributedString::decode(&ObjectValue::Ref(obj)).unwrap();
    let eq = NSMutableAttributedString {
        nsattribute_info: vec![5, 0, 11, 1].into(),
        nsattributes: vec![
            HashMap::from([(
                "NSColor".into(),
                NSColor {
                    nscolor_space: 1,
                    nscomponents: vec![49, 32, 48, 32, 48, 32, 49].into(),
                    nsrgb: vec![
                        48, 46, 57, 56, 53, 57, 53, 52, 49, 54, 53, 53, 32, 48, 32, 48, 46, 48, 50,
                        54, 57, 52, 48, 48, 48, 56, 54, 51, 0,
                    ]
                    .into(),
                    nscustom_color_space: Foo { nsid: 7 },
                    my_field: "".into(),
                },
            )]),
            HashMap::new(),
        ],
        nsstring: "firstsecondthird".into(),
    };
    assert_eq!(decoded, eq);
}

#[test]
fn simple_dict_derive() {
    let unarchiver = KeyedArchive::from_file("./tests_resources/plists/simpleDict.plist").unwrap();
    let root = unarchiver.root().unwrap();
    let decoded_data = HashMap::<String, DictMember>::decode(&root.into()).unwrap();
    let dict = HashMap::from([
        (
            "First key".to_string(),
            DictMember::String("First value".to_string()),
        ),
        (
            "Second key".to_string(),
            DictMember::String("Second value".to_string()),
        ),
        ("Array key".to_string(), DictMember::IntArray(vec![1, 2, 3])),
    ]);
    assert_eq!(decoded_data, dict);
}
