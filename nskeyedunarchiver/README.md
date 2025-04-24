# nskeyedunarchiver

Decodes Cocoa Keyed Archive into native Rust structures

## What is Cocoa Keyed Archive?

**Cocoa Keyed Archive** is a serialized Objective C object created by the [`NSKeyedArchiver`](https://developer.apple.com/documentation/foundation/nskeyedarchiver?language=objc) class. Such objects should implement [`NSCoding`](https://developer.apple.com/documentation/foundation/nscoding?language=objc) protocol. An archive is actually a binary `.plist` with a specific structure. Deserializing an archive in Objective C results in a freshly created objects with their fields being set.

## Keyed Archive Structure

By itself a keyed archive is a plist dictionary. For us the `$objects` and `$top` keys are important. The `$top` is an entry point of any data. It contains references (Uids) to objects of `$objects`. Decoding process is starting here.

The `$objects` key is an array of encoded objects. They're represented as `ValueRef`. Object is a plist dictionary as well, containing plain plist types and references to other objects.

## Deserializing in Rust

Decoding a Keyed Archive in Rust goes in 3 steps:

1. Reading and deserializing a plist using the `plist` crate.
2. Creating an internal representation of archive objects, including references to other objects using reference-counting pointers.
3. Initializing Rust structures with decoded data.

Currently the process involves Rc pointers. When dealing with circular references the data is not going to be dropped, so memory leaks occur.

## Decodable trait

To make a Rust structure decodable from a Keyed Archive it should implement the `Decodable` trait.

The trait is already implemented for these types:

|Plist value or class|Rust type|
|--|--|
|String (+ref*), NSString, NSMutableString|String|
|Integer (+ref)|Integer, u8, u16, u32, u64, i8, i16, i32, i64|
|Real (+ref)|f64|
|Boolean (+ref)|bool|
|Data (+ref), NSData|Data|
|NSArray, NSMutableArray, NSSet, NSMutableSet|Vec\<T\> where T: Decodable|
|NSDictionary, NSMutableDictionary|HashMap\<K, V\> where K: Decodable + Hash + Eq, V: Decodable|
|Uid (a reference)|ValueRef|

*`+ref` means that it either may be a plain plist value or a reference to it*

The `decode` method of the trait receives an `ObjectValue` that represents any possible value inside of objects. Usually `ObjectValue::Ref` variant is what you need for your types. Other variants may be used if you want to create a type that redefines any implementations. For instance, you may create a type that works with `ObjectValue::String` and returns itself, consisting some parsed data.

You may find manual `Decodable` implementations in the `tests/simple_test.rs`.

## #[Decodable] macro

The easiest way to make a type `Decodable` is to derive the `Decodable` *macro* for your struct or enum. Types of fields and variants should also implement `Decodable` trait.

See [the readme](../nskeyedunarchiver_derive/README.md) of the `nskeyedunarchiver_derive` crate.

## Example

The following example shows an Objective C object being encoded to a keyed archive and decoded in Rust.

Objective C:

```objectivec
@interface Note : NSObject <NSCoding> {
  NSString *title;
  NSString *author;
  BOOL published;
  // This array contains only a string, an integer and a boolean
  NSArray *array;
}

@implementation  Note
  /* some code is omitted */

- (void)encodeWithCoder:(NSCoder *)encoder {
  [encoder encodeObject:title forKey:@"title"];
  [encoder encodeObject:author forKey:@"author"];
  [encoder encodeInt:date forKey:@"date"];
  [encoder encodeBool:published forKey:@"published"];
  [encoder encodeObject:array forKey:@"array"];
}

@end
```

Rust:

```rust
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
    #[decodable(default, rename = "published")]
    is_published: bool,
    array: Vec<ArrayMember>,
    // It will be `None`
    not_present: Option<String>,
    #[decodable(unhandled)]
    // The `date` field will go here
    unhandled: HashMap<String, ObjectValue>,
}

fn note() {
    let archive = KeyedArchive::from_file("./tests_resources/plists/note.plist").unwrap();
    let obj = archive.root().unwrap();
    let decoded = Note::decode(&obj.into()).unwrap();
}
```

The full code is available at `./tests/derive.rs` and `./tests_resources/main.m`.
