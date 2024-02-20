mod impls;
pub use impls::*;

use std::any::{Any, TypeId};
use crate::{DeError, ObjectRef};

pub trait Decodable: std::fmt::Debug + Sized {
    fn class() -> Option<&'static str>;
    fn decode(object: ObjectRef, types: &[ObjectAny]) -> Result<Self, DeError>;
    fn decode_as_any(object: ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn std::any::Any>, DeError> where Self: 'static {
        Ok(Box::new(Self::decode(object, types)?) as Box<dyn std::any::Any>)
    }
    fn as_object_any() -> ObjectAny where Self: 'static {
        ObjectAny::new(
            TypeId::of::<Self>(),
            Self::class,
            Self::decode_as_any
        )
    }
}

pub type ObjectClassFn = fn () -> Option<&'static str>;
pub type ObjectDecodeFn = fn (obj: ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn Any>, DeError>;
pub type ObjectType = (TypeId, ObjectClassFn, ObjectDecodeFn);
pub type ObjectTypes = Vec<ObjectType>;

pub struct ObjectAny(TypeId, ObjectClassFn, ObjectDecodeFn);
impl ObjectAny {
    pub fn new(t: TypeId, c: ObjectClassFn, d: ObjectDecodeFn) -> Self {
        Self(t, c, d)
    }
    pub fn type_id(&self) -> TypeId {self.0}
    pub fn class(&self) -> Option<&'static str> {self.1()}
    pub fn decode(&self, obj: ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn Any>, DeError> {self.2(obj, types)}
}

#[macro_export]
macro_rules! make_types {
    ($($name:ident),*) => {
        {
            Vec::from([
                $crate::de::NSArray::as_object_any(),
                $crate::de::NSDictionary::as_object_any(),
                $(
                    $name::as_object_any()
                ),*
            ])
        }
    };
}

#[macro_export]
macro_rules! get_object {
    ($obj_ref:ident) => {
        {
            let Some(obj) = $obj_ref.as_object() else {
                return Err($crate::DeError::ExpectedObject);
            };
            obj
        }
    };
}

pub fn decode_any_object(object_ref: ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn Any>, DeError> {
    let Some(object) = object_ref.as_object() else {
        return Err(DeError::ExpectedObject);
    };
    let class = object.class();
    let mut result = None;
    for typ in types {
        if typ.class().unwrap() == &class {
            result = Some(
                typ.decode(object_ref.clone(), types)
            );
        }
    }
    match result {
        Some(val) => {
            val
        },
        None => {
            Err(DeError::Message(format!("Undecodable object: {class}")))
        },
    }
}
