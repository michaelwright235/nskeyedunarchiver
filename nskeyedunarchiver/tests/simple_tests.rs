use std::collections::HashMap;
use std::rc::{Rc, Weak};

use nskeyedunarchiver::de::Decodable;
use nskeyedunarchiver::{object_types, ArchiveValue, Decodable, NSKeyedUnarchiver, ValueRef};

const PLIST_PATH: &str = "./tests_resources/plists/";

fn open_file(name: &str) -> (ValueRef, Vec<Weak<ArchiveValue>>) {
    let unarchiver = NSKeyedUnarchiver::from_file(format!("{PLIST_PATH}{name}")).unwrap();
    let weak_refs: Vec<Weak<ArchiveValue>> = unarchiver
        .values()
        .iter()
        .map(|v| Rc::downgrade(v))
        .collect();
    (unarchiver.top()["root"].clone(), weak_refs)
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
    let decoded_string = String::decode(&root.into(), &object_types!()).unwrap();
    check_rc_strong_count(&weak_refs);
    assert_eq!(decoded_string, "Some string!");
}

#[derive(Decodable, PartialEq, Debug)]
enum SimpleArrayItem {
    String(String),
    Array(Vec<String>)
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
    let decoded_data = Vec::<SimpleArrayItem>::decode(&root.into(), &object_types!()).unwrap();
    let simple_array = vec![
        SimpleArrayItem::String("value1".into()),
        SimpleArrayItem::String("value2".into()),
        SimpleArrayItem::Array(vec!["innerValue3".into(), "innerValue4".into()])
    ];
    assert_eq!(decoded_data, simple_array);

    check_rc_strong_count(&weak_refs);
}

#[derive(Decodable, PartialEq, Debug)]
enum SimpleDictItem {
    String(String),
    Array(Vec<i64>)
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
    let decoded_data = HashMap::<String, SimpleDictItem>::decode(&root.into(), &object_types!()).unwrap();
    let simple_dict: HashMap<String, SimpleDictItem> = HashMap::from([
        ("First key".into(), SimpleDictItem::String("First value".into())),
        ("Second key".into(), SimpleDictItem::String("Second value".into())),
        ("Array key".into(), SimpleDictItem::Array(vec![1, 2, 3])),
    ]);
    assert_eq!(decoded_data, simple_dict);
    check_rc_strong_count(&weak_refs);
}

#[test]
fn ns_data() {
    let (root, weak_refs) = open_file("nsData.plist");
    let decoded_data = Vec::<u8>::decode(&root.into(), &object_types!()).unwrap();
    let ns_data = "Some data!".as_bytes();
    assert_eq!(decoded_data, ns_data);
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
