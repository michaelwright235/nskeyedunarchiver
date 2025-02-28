#![allow(unused_variables, dead_code, non_snake_case)]
#![cfg(feature = "derive")]

use keyed_archive_derive::Decodable;
use nskeyedunarchiver::{
    de::{Decodable, NSData, NSDictionary},
    object_types, NSKeyedUnarchiver};

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

#[test]
fn nsaffine_transform() {
    let unarchiver =
        NSKeyedUnarchiver::from_file("./tests_resources/plists/NSAffineTransform.plist").unwrap();
    let obj = unarchiver.top().get("root").unwrap().clone();
    let decoded = NSAffineTransform::decode(obj, &vec![]).unwrap();
    println!("{decoded:?}")
}

#[test]
fn nsmutable_attributed_string() {
    let unarchiver =
        NSKeyedUnarchiver::from_file("./tests_resources/plists/NSMutableAttributedString.plist")
            .unwrap();
    let obj = unarchiver.top().get("root").unwrap().clone();
    let decoded = NSMutableAttributedString::decode(obj, &object_types!(NSColor)).unwrap();
    println!("{decoded:#?}")
}
