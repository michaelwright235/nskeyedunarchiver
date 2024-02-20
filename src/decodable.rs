

#[cfg(test)]
mod test {
    use crate::DeError;
    use crate::Decodable;
    use crate::NSArray;
    use crate::NSKeyedUnarchiver;
    use crate::ObjectAny;
    use crate::ObjectRef;
    use simplelog::{Config, LevelFilter, SimpleLogger};
    use log::debug;

    #[derive(Debug)]
    struct NSAffineTransform {
        data: Vec<u8>,
        b: bool,
        i: i64,
        u: u64
    }
    impl Decodable for NSAffineTransform {
        fn class() -> Option<&'static str> {
            Some("NSAffineTransform")
        }

        fn decode(object: ObjectRef, _types: &[ObjectAny]) -> Result<Self, DeError> {
            let obj = get_object!(object);
            let data = obj.decode_data("NSTransformStruct")?.to_vec();
            let b = obj.decode_bool("Boolean")?;
            let i = obj.decode_i64("INumber")?;
            let u = obj.decode_u64("UNumber")?;
            Ok(Self {
                data, b, i ,u
            })
        }
    }
    make_decodable!(NSAffineTransform);

    #[test]
    fn detest() {
        let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());
        let unarchiver = NSKeyedUnarchiver::from_file("./arrays.plist").unwrap();
        let result = NSArray::decode(
            unarchiver.top()["root"].clone(),
            &make_types!()
        );
        debug!("{:#?}", result);
    }

    #[test]
    fn detest2() {
        let _ = SimpleLogger::init(LevelFilter::Debug, Config::default());
        let unarchiver = NSKeyedUnarchiver::from_file("./NSAffineTransform3.plist").unwrap();
        let result = NSAffineTransform::decode(
            unarchiver.top()["root"].clone(),
            &make_types!(NSAffineTransform)
        );
        debug!("{:#?}", result);
    }
}
