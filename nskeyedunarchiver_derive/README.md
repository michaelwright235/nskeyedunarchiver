# nskeyedunarchiver_derive

Derive macro which generates an impl of the trait `Decodable`.

## #[Decodable] macro

The easiest way to make a type `Decodable` is to derive the `Decodable` *macro* for your struct or enum. Types of fields and variants should also implement `Decodable` trait.

The macro attribute is `#[decodable(...)]`. Every inner attribute is separated with a comma `,`.

### Container attributes

- `#[decodable(rename = "...")]`: decodes a container with the given name instead of its Rust name.

### Field and Variant attributes

- `#[decodable(rename = "...")]`: decodes a field or variant with the given name instead of its Rust name.
- `#[decodable(skip)]`: do not decode a field or variant. Doesn't work with other attributes.
- `#[decodable(default)]`: if the value is not present when decoding, use the `Default::default()`.

### Field attributes only

- `#[decodable(unhandled)]`: creates a hashmap of any values that are unhandled and thus hasn't been decoded. A field should have a type of `HashMap<String, ObjectValue>`.
