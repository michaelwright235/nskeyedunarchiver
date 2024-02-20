mod impls;
pub use impls::*;

use std::any::{Any, TypeId};
use crate::{DeError, ObjectRef};

pub trait Decodable: std::fmt::Debug {
    fn class() -> Option<&'static str> where Self: Sized;
    fn decode(object: ObjectRef, types: &[ObjectAny]) -> Result<Self, DeError> where Self:Sized;
}

pub type ObjectClassFn = fn () -> &'static str;
pub type ObjectDecodeFn = fn (obj: ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn Any>, DeError>;
pub type ObjectType = (TypeId, ObjectClassFn, ObjectDecodeFn);
pub type ObjectTypes = Vec<ObjectType>;

pub struct ObjectAny(TypeId, ObjectClassFn, ObjectDecodeFn);
impl ObjectAny {
    pub fn new(t: TypeId, c: ObjectClassFn, d: ObjectDecodeFn) -> Self {
        Self(t, c, d)
    }
    pub fn type_id(&self) -> TypeId {self.0}
    pub fn class(&self) -> &'static str {self.1()}
    pub fn decode(&self, obj: ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn Any>, DeError> {self.2(obj, types)}
}

#[macro_export]
macro_rules! make_decodable {
    ($vis:vis $name:ident) => {
        $crate::paste::paste! {
            #[doc(hidden)]
            use $name as [< _$name _typ >];
            #[doc(hidden)]
            #[allow(unused)]
            $vis mod [<$name:lower _helper>] {
                use core::any::{Any, TypeId};
                use $crate::de::Decodable;
                use $crate::de::ObjectAny;
                use super::[< _$name _typ >] as $name;

                #[doc(hidden)]
                pub fn [<_ $name:lower _class>]() -> &'static str {
                    $name::class().unwrap()
                }
                #[doc(hidden)]
                pub fn [<_ $name:lower _decode>](obj: $crate::ObjectRef, types: &[ObjectAny]) -> Result<Box<dyn Any>, $crate::DeError> {
                    $name::decode(obj, types).map(|o| Box::new(o) as Box<dyn Any>)
                }
                #[doc(hidden)]
                pub fn [<$name:lower _object_type>]() -> ObjectAny {
                    ObjectAny::new(
                        TypeId::of::<$name>(),
                        [<_ $name:lower _class>],
                        [<_ $name:lower _decode>]
                    )
                }
            }
        }
    };
}

#[macro_export]
macro_rules! make_types {
    ($($name:ident),*) => {
        $crate::paste::paste! {{
            Vec::from([
                $crate::de::nsarray_helper::nsarray_object_type(),
                $crate::de::nsdictionary_helper::nsdictionary_object_type(),
                $(
                    [<$name:lower _helper>]::[<$name:lower _object_type>]()
                ),*
            ])
        }}
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
        if typ.class() == &class {
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
