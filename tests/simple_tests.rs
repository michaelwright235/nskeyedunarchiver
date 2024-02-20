use nskeyedunarchiver::de::{Decodable, NSArray, NSData, NSDictionary, ObjectType};
use nskeyedunarchiver::{as_object, object_types, DeError, Integer, NSKeyedUnarchiver, ObjectRef};

const PLIST_PATH: &str = "./tests_resources/plists/";

fn open_file(name: &str) -> ObjectRef {
    let unarchiver = NSKeyedUnarchiver::from_file(format!("{PLIST_PATH}{name}")).unwrap();
    unarchiver.top()["root"].clone()
}
#[test]
fn plain_string() {
    // -- String: "Some string!"
    let root = open_file("plainString.plist");
    let decoded_string = String::decode(root, &object_types!()).unwrap();
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

    let root = open_file("simpleArray.plist");
    let mut decoded_data = NSArray::decode(root, &object_types!()).unwrap();
    let parent0: Box<String> = decoded_data.remove(0).downcast().unwrap();
    assert_eq!(parent0.as_str(), "value1");
    let parent1: Box<String> = decoded_data.remove(0).downcast().unwrap();
    assert_eq!(parent1.as_str(), "value2");

    let mut parent2: Box<NSArray> = decoded_data.remove(0).downcast().unwrap();
    let child1: Box<String> = parent2.remove(0).downcast().unwrap();
    assert_eq!(child1.as_str(), "innerValue3");
    let child2: Box<String> = parent2.remove(0).downcast().unwrap();
    assert_eq!(child2.as_str(), "innerValue4");
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

    let root = open_file("simpleDict.plist");
    let mut decoded_data = NSDictionary::decode(root, &object_types!()).unwrap();

    let value1: Box<String> = decoded_data
        .remove("First key")
        .unwrap()
        .downcast()
        .unwrap();
    assert_eq!(value1.as_str(), "First value");

    let value2: Box<String> = decoded_data
        .remove("Second key")
        .unwrap()
        .downcast()
        .unwrap();
    assert_eq!(value2.as_str(), "Second value");

    let value3: Vec<Box<Integer>> = (decoded_data
        .remove("Array key")
        .unwrap()
        .downcast()
        .unwrap() as Box<NSArray>)
        .into_inner()
        .into_iter()
        .map(|v| v.downcast().unwrap() as Box<Integer>)
        .collect();

    assert_eq!(value3[0].as_unsigned().unwrap(), 1);
    assert_eq!(value3[1].as_unsigned().unwrap(), 2);
    assert_eq!(value3[2].as_unsigned().unwrap(), 3);
}

#[test]
fn ns_data() {
    let root = open_file("nsData.plist");
    let decoded_data = NSData::decode(root, &object_types!(NSData)).unwrap();
    let s = String::from_utf8(decoded_data.into_inner()).unwrap();
    assert_eq!(s, "Some data!");
}
