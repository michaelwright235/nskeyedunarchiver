use nskeyedunarchiver::{as_object, object_types, DeError, NSKeyedUnarchiver, ObjectRef, Integer};
use nskeyedunarchiver::de::{Decodable, NSArray, NSDictionary, ObjectType};

const PLIST_PATH: &str = "./tests_resources/plists/";

fn open_file(name: &str) -> ObjectRef {
    let unarchiver = NSKeyedUnarchiver::from_file(
        format!("{PLIST_PATH}{name}")
    ).unwrap();
    unarchiver.top()["root"].clone()
}
#[test]
fn plain_string() {
    let root = open_file("plainString.plist");
    let decoded_string = String::decode(root, &object_types!()).unwrap();
    assert_eq!(decoded_string, "Some string!");
}

#[test]
fn simple_array() {
    // -- NSArray
    //    -- String: value1
    //    -- String: value2
    //    -- NSArray
    //       -- String: innerValue3
    //       -- String: innerValue4

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
    let root = open_file("simpleDict.plist");
    let mut decoded_data = NSDictionary::decode(root, &object_types!()).unwrap();

    let value1: Box<String> = decoded_data.remove("First key").unwrap().downcast().unwrap();
    assert_eq!(value1.as_str(), "First value");

    let value2: Box<String> = decoded_data.remove("Second key").unwrap().downcast().unwrap();
    assert_eq!(value2.as_str(), "Second value");

    let mut value3: Box<NSArray> = decoded_data.remove("Array key").unwrap().downcast().unwrap();

    let value3_child1: Box<Integer> = value3.remove(0).downcast().unwrap();
    assert_eq!(value3_child1.as_unsigned().unwrap(), 1);

    let value3_child2: Box<Integer> = value3.remove(0).downcast().unwrap();
    assert_eq!(value3_child2.as_unsigned().unwrap(), 2);

    let value3_child3: Box<Integer> = value3.remove(0).downcast().unwrap();
    assert_eq!(value3_child3.as_unsigned().unwrap(), 3);
}

#[test]
fn ns_data() {
    struct NSData(Vec<u8>);

    impl Decodable for NSData {
        fn is_type_of(classes: &[String]) -> bool {
            classes[0] == "NSData" || classes[0] == "NSMutableData"
        }

        fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError> {
            let obj = as_object!(object);
            let data = obj.decode_data("NS.data")?.to_vec();
            Ok(Self(data))
        }
    }

    let root = open_file("nsData.plist");
    let decoded_data = NSData::decode(root, &object_types!(NSData)).unwrap();
    let s = String::from_utf8(decoded_data.0).unwrap();
    assert_eq!(s, "Some data!");
}

#[derive(Debug)]
struct NSAffineTransform<'a> {
    data: Vec<u8>,
    p: Option<&'a str>,
}

impl Decodable for NSAffineTransform<'_> {
    fn is_type_of(classes: &[String]) -> bool {
        classes[0] == "NSAffineTransform"
    }

    fn decode(object: ObjectRef, _types: &[ObjectType]) -> Result<Self, DeError>
    {
        let obj = as_object!(object);
        let data = obj.decode_data("NSTransformStruct")?.to_vec();
        Ok(Self { data, p: None })
    }
}
