use plist::Integer;
use super::{Decodable, NSArray, ObjectType};
use crate::{DeError, Object, UniqueId, ValueRef};

pub trait ObjectMember {
    fn get_from_object(
        obj: &Object,
        key: &str,
        types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static;

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static;
}

impl ObjectMember for String {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_string(key).map(|v| v.into_owned())
    }

    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        Some(ObjectType::new::<Self>())
    }
}

impl ObjectMember for f64 {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_float(key).copied()
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        Some(ObjectType::new::<Self>())
    }
}

impl ObjectMember for u64 {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_integer(key).and_then(|v| {
            v.as_unsigned().ok_or(DeError::Message(
                "Unable to represent an integer as u64".into(),
            ))
        })
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}

impl ObjectMember for i64 {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_integer(key).and_then(|v| {
            v.as_signed().ok_or(DeError::Message(
                "Unable to represent an integer as i64".into(),
            ))
        })
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}

impl ObjectMember for Integer {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_integer(key).copied()
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        Some(ObjectType::new::<Self>())
    }
}

macro_rules! impl_object_member {
    ($($t:ty),+) => {
        $(
            impl ObjectMember for $t {
                fn get_from_object(
                    obj: &Object,
                    key: &str,
                    types: &[ObjectType],
                ) -> std::result::Result<Self, DeError>
                where
                    Self: Sized + 'static {
                        obj.decode_object_as::<Self>(key, types)
                }
                fn as_object_type() -> Option<ObjectType>
                where
                    Self: Sized+ 'static {
                    Some(ObjectType::new::<Self>())
                }
            }
        )+
    };
}

impl_object_member!(super::NSArray, super::NSSet, super::NSDictionary, super::NSData);

impl ObjectMember for bool {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_bool(key)
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}

impl ObjectMember for Vec<u8> {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_data(key).map(|v| v.to_vec())
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}

impl<T: Decodable> ObjectMember for Vec<T> {
    fn get_from_object(
        obj: &Object,
        key: &str,
        types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static,
    {
        let array = obj.decode_object(key)?;
        let obj = array.as_object().ok_or(DeError::ExpectedObject)?;
        if !NSArray::is_type_of(obj.classes()) {
            return Err(DeError::Message(
                "NSArray: not an array".to_string(),
            ));
        }
        let Ok(inner_objs) = obj.decode_array("NS.objects") else {
            return Err(DeError::Message(
                "NSArray: Expected array of objects".to_string(),
            ));
        };
        let mut result = Vec::with_capacity(inner_objs.len());
        for inner_obj in inner_objs {
            result.push(T::decode(inner_obj.clone(), types)?);
        }

        /*let arr = NSArray::get_from_object(obj, key, types)?;
        arr.try_into_objects::<T>()*/
        Ok(result)
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}



impl ObjectMember for ValueRef {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_object(key)
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}

impl ObjectMember for UniqueId {
    fn get_from_object(
        obj: &Object,
        key: &str,
        _types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        obj.decode_object(key).map(|v| v.unique_id)
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}

impl<T: ObjectMember> ObjectMember for Option<T> {
    fn get_from_object(
        obj: &Object,
        key: &str,
        types: &[ObjectType],
    ) -> std::result::Result<Self, DeError>
    where
        Self: Sized + 'static {
        Ok(T::get_from_object(obj, key, types).ok())
    }
    fn as_object_type() -> Option<ObjectType>
    where
        Self: Sized+ 'static {
        None
    }
}
