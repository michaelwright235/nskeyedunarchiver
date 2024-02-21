use std::rc::{Rc, Weak};

use nskeyedunarchiver::de::{Decodable, NSArray, NSData, NSDictionary};
use nskeyedunarchiver::{object_types, ArchiveValue, Integer, NSKeyedUnarchiver, ValueRef};

const PLIST_PATH: &str = "./tests_resources/plists/";

fn open_file(name: &str) -> (ValueRef, Vec<Weak<ArchiveValue>>) {
    let unarchiver = NSKeyedUnarchiver::from_file(format!("{PLIST_PATH}{name}")).unwrap();
    let weak_refs: Vec<Weak<ArchiveValue>> = unarchiver
        .objects()
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
    // -- String: "Some string!"=
    let (root, weak_refs) = open_file("plainString.plist");
    let decoded_string = String::decode(root, &object_types!()).unwrap();
    check_rc_strong_count(&weak_refs);
    assert_eq!(decoded_string, "Some string!");
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
    let mut decoded_data = NSArray::decode(root, &object_types!()).unwrap();
    let parent0: Box<String> = decoded_data.remove_as_object::<String>(0).unwrap();
    assert_eq!(parent0.as_str(), "value1");
    let parent1: Box<String> = decoded_data.remove_as_object::<String>(0).unwrap();
    assert_eq!(parent1.as_str(), "value2");

    let parent2: Vec<Box<String>> = decoded_data
        .remove_as_object::<NSArray>(0)
        .unwrap()
        .try_into_objects::<String>()
        .unwrap();
    assert_eq!(parent2[0].as_str(), "innerValue3");
    assert_eq!(parent2[1].as_str(), "innerValue4");
    check_rc_strong_count(&weak_refs);
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
    let mut decoded_data = NSDictionary::decode(root, &object_types!()).unwrap();

    let value1: Box<String> = decoded_data
        .remove_as_object::<String>("First key")
        .unwrap();
    assert_eq!(value1.as_str(), "First value");

    let value2: Box<String> = decoded_data
        .remove_as_object::<String>("Second key")
        .unwrap();
    assert_eq!(value2.as_str(), "Second value");

    let value3: Vec<Box<Integer>> = decoded_data
        .remove_as_object::<NSArray>("Array key")
        .unwrap()
        .try_into_objects::<Integer>()
        .unwrap();

    assert_eq!(value3[0].as_unsigned().unwrap(), 1);
    assert_eq!(value3[1].as_unsigned().unwrap(), 2);
    assert_eq!(value3[2].as_unsigned().unwrap(), 3);
    check_rc_strong_count(&weak_refs);
}

#[test]
fn ns_data() {
    let (root, weak_refs) = open_file("nsData.plist");
    let decoded_data = NSData::decode(root, &object_types!(NSData)).unwrap();
    let s = String::from_utf8(decoded_data.into_inner()).unwrap();
    assert_eq!(s, "Some data!");
    check_rc_strong_count(&weak_refs);
}

#[test]
fn circular_reference() {
    // -- NSMutableArray   <-|
    //    -- NSMutableArray -^
    let (root, weak_refs) = open_file("circularReference.plist");
    std::mem::drop(root);
    check_rc_strong_count(&weak_refs);
}
