mod impls;
pub use impls::*;

use crate::{DeError, ObjectRef};
use std::any::{Any, TypeId};

pub trait Decodable: Sized {
    fn is_type_of(classes: &[String]) -> bool;
    fn decode(object: ObjectRef, types: &[ObjectType]) -> Result<Self, DeError>;
    fn decode_as_any(
        object: ObjectRef,
        types: &[ObjectType],
    ) -> Result<Box<dyn std::any::Any>, DeError>
    where
        Self: 'static,
    {
        Ok(Box::new(Self::decode(object, types)?) as Box<dyn std::any::Any>)
    }
    fn as_object_type() -> ObjectType
    where
        Self: 'static,
    {
        ObjectType::new(TypeId::of::<Self>(), Self::is_type_of, Self::decode_as_any)
    }
}

type IsTypeOfFn = fn(classes: &[String]) -> bool;
type DecodeAsAnyFn = fn(obj: ObjectRef, types: &[ObjectType]) -> Result<Box<dyn Any>, DeError>;

pub struct ObjectType(TypeId, IsTypeOfFn, DecodeAsAnyFn);
impl ObjectType {
    pub fn new(t: TypeId, c: IsTypeOfFn, d: DecodeAsAnyFn) -> Self {
        Self(t, c, d)
    }
    pub fn type_id(&self) -> TypeId {
        self.0
    }
    pub fn is_type_of(&self, classes: &[String]) -> bool {
        self.1(classes)
    }
    pub fn decode(&self, obj: ObjectRef, types: &[ObjectType]) -> Result<Box<dyn Any>, DeError> {
        self.2(obj, types)
    }
}

#[macro_export]
macro_rules! object_types {
    ($($name:ident),*) => {
        Vec::from([
            $crate::de::NSArray::as_object_type(),
            $crate::de::NSDictionary::as_object_type(),
            $(
                $name::as_object_type()
            ),*
        ])
    };
}

#[macro_export]
macro_rules! as_object {
    ($obj_ref:ident) => {{
        let Some(obj) = $obj_ref.as_object() else {
            return Err($crate::DeError::ExpectedObject);
        };
        obj
    }};
}

pub fn object_ref_to_any(
    object_ref: ObjectRef,
    types: &[ObjectType],
) -> Result<Box<dyn Any>, DeError> {
    let Some(object) = object_ref.as_object() else {
        return Err(DeError::ExpectedObject);
    };
    let classes = object.classes();
    let mut result = None;
    for typ in types {
        if typ.is_type_of(&classes) {
            result = Some(typ.decode(object_ref.clone(), types));
        }
    }
    match result {
        Some(val) => val,
        None => Err(DeError::Message(format!(
            "Undecodable object: {}",
            classes[0]
        ))),
    }
}
