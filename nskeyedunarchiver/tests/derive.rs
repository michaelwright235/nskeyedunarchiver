#![allow(unused_variables, dead_code, non_snake_case)]
#![cfg(feature = "derive")]

use std::collections::HashMap;

use nskeyedunarchiver::{
    de::{Decodable, NSData, NSDictionary},
    object_types, NSKeyedUnarchiver, ObjectValue,
};
use nskeyedunarchiver_derive::Decodable;

#[derive(Decodable, Debug)]
struct NSAffineTransform {
    NSTransformStruct: Option<Vec<u8>>,
}

#[derive(Decodable, Debug)]
struct NSMutableAttributedString {
    NSAttributeInfo: NSData,
    NSAttributes: Vec<NSDictionary>,
    NSString: String,
}

#[derive(Decodable, Debug)]
struct NSColor {
    NSColorSpace: i64,
    NSComponents: Vec<u8>,
    NSRGB: Vec<u8>,
    NSCustomColorSpace: Foo,
    #[decodable(skip)] // Default::default()
    my_field: String,
}

#[derive(Decodable, Debug)]
#[decodable(rename = "NSColorSpace")]
struct Foo {
    #[decodable(rename = "NSICC")]
    icc: Vec<u8>,
    NSID: i64,
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
    let unarchiver = NSKeyedUnarchiver::from_file("./tests_resources/plists/note.plist").unwrap();
    let obj = unarchiver.top().get("root").unwrap().clone();
    let decoded = Note::decode(&ObjectValue::Ref(obj), &object_types!()).unwrap();

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

#[test]
fn nsaffine_transform() {
    let unarchiver =
        NSKeyedUnarchiver::from_file("./tests_resources/plists/NSAffineTransform.plist").unwrap();
    let obj = unarchiver.top().get("root").unwrap().clone();
    let decoded = NSAffineTransform::decode(&ObjectValue::Ref(obj), &vec![]).unwrap();
    println!("{decoded:?}")
}

#[test]
fn nsmutable_attributed_string() {
    let unarchiver =
        NSKeyedUnarchiver::from_file("./tests_resources/plists/NSMutableAttributedString.plist")
            .unwrap();
    let obj = unarchiver.top().get("root").unwrap().clone();
    let decoded = NSMutableAttributedString::decode(&ObjectValue::Ref(obj), &object_types!(NSColor)).unwrap();
    println!("{decoded:#?}")
}
