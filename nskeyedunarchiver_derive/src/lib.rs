use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};
use syn::{spanned::Spanned, Error, Result};

// All possible attributes
// #[decodable(rename = "foo")], #[decodable(skip)]
const BOOL_ATTRS: [&str; 1] = ["skip"];
const STR_ATTRS: [&str; 1] = ["rename"];

#[derive(Debug, Default)]
struct MacroAttributes {
    str_attrs: HashMap<String, String>,
    bool_attrs: Vec<String>
}

impl TryFrom<&[syn::Attribute]> for MacroAttributes {
    type Error = syn::Error;

    fn try_from(value: &[syn::Attribute]) -> std::result::Result<Self, Self::Error> {
        let mut decodable_attr = None;
        for attr in value {
            if attr.path().get_ident().unwrap() == "decodable" {
                decodable_attr = Some(attr);
            }
        }
        let Some(decodable_attr) = decodable_attr else {
                return Ok(Self::default());
        };
        let syn::Meta::List(list) = &decodable_attr.meta else {
            return Err(Error::new(decodable_attr.path().span(), "Unable to parse attributes"));
        };
        let raw_attr = list.tokens.to_string();
        let mut str_attrs = HashMap::new();
        let mut bool_attrs = Vec::new();
        for param in raw_attr.split(",") {
            let mut pair: Vec<&str> = param.split("=").collect();
            for s in &mut pair {
                *s = s.trim();
            }
            if pair.len() == 1 {
                if bool_attrs.contains(&pair[0].to_string()) {
                    return Err(Error::new(decodable_attr.path().span(), "An attribute cannot be set more than once"));
                }
                if !BOOL_ATTRS.contains(&pair[0]) {
                    return Err(Error::new(decodable_attr.path().span(), format!("Unknown attribute `{}`", pair[0])));
                }
                bool_attrs.push(pair[0].to_string());
                continue;
            }
            if pair.len() != 2 {
                return Err(Error::new(decodable_attr.path().span(), "Unknown attribute"));
            }
            let attr_name = pair[0];
            let attr_value = pair[1];
            if !attr_value.starts_with("\"") || !attr_value.ends_with("\"") {
                return Err(Error::new(decodable_attr.path().span(), "An attribute value should start and end with a quote"));
            }
            let attr_value = &attr_value[1..attr_value.len()-1]; // remove quotes ""
            if str_attrs.contains_key(pair[0]) {
                return Err(Error::new(decodable_attr.path().span(), "An attribute cannot be set more than once"));
            }
            if !STR_ATTRS.contains(&pair[0]) {
                return Err(Error::new(decodable_attr.path().span(), format!("Unknown attribute `{}`", pair[0])));
            }
            str_attrs.insert(attr_name.to_string(), attr_value.to_string());
        }
        if bool_attrs.contains(&"skip".to_string()) && !str_attrs.is_empty() {
            return Err(Error::new(decodable_attr.path().span(), "`skip` cannot be used with other arguments"));
        }
        Ok(Self {
            str_attrs,
            bool_attrs
        })
    }
}

fn decodable_impl(input: DeriveInput) -> Result<TokenStream> {
    let syn::Data::Struct(cur_struct) = &input.data else {
        return Err(Error::new(
            input.span(),
            "Only structs are supported",
        ));
    };

    let syn::Fields::Named(named_fields) = &cur_struct.fields else {
        return Err(Error::new(
            input.span(),
            "Only structs with named fields are supported",
        ));
    };

    let struct_ident = &input.ident;
    let mut struct_name = struct_ident.to_string();

    let struct_attrs = MacroAttributes::try_from(input.attrs.as_slice())?;

    if let Some(new_name) = struct_attrs.str_attrs.get("rename") {
        struct_name = new_name.to_string();
    }
    if struct_attrs.bool_attrs.contains(&"skip".to_string()) {
        return Err(Error::new(input.attrs[0].path().span(), "`skip` can only be used for fields"));
    }

    let mut field_inits: Vec<proc_macro2::TokenStream> =
        Vec::with_capacity(named_fields.named.len());

    let mut object_types = Vec::with_capacity(named_fields.named.len());
    for f in &named_fields.named {
        // hangle things like Vec<u8> (brackets like <u8>)
        let field_attrs = MacroAttributes::try_from(f.attrs.as_slice())?;
        if field_attrs.bool_attrs.contains(&"skip".to_string()) {
            continue;
        }
        if let syn::Type::Path(b) = &f.ty {
            let last_segment = b.path.segments.last().unwrap();
            let last_segment_ident = &last_segment.ident;
            if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                let a = args.to_token_stream();
                let inner = quote! {
                    #last_segment_ident::#a
                };
                object_types.push(inner);
                continue;
            }
        }

        // regular types
        object_types.push(f.ty.to_token_stream());
    }
    let object_types_macro = quote! {
        {
            let mut v: Vec<ObjectType> = vec![];
            #(
                if let Some(t) = <#object_types as ObjectMember>::as_object_type() {
                    v.push(t);
                }
            )*
            v
        }
    };
    //eprintln!("{object_types_macro}");

    for f in &named_fields.named {
        let mut field_name = f.ident.as_ref().unwrap().to_string();
        let field_ident = format_ident!("{field_name}");
        let field_type = &f.ty;
        let mut found_skip = false;

        let field_attrs = MacroAttributes::try_from(f.attrs.as_slice())?;

        if let Some(new_name) = field_attrs.str_attrs.get("rename") {
            field_name = new_name.to_string();
        }
        if field_attrs.bool_attrs.contains(&"skip".to_string()) {
            found_skip = true;
        }

        if found_skip {
            let inner = quote! {
                #field_ident: Default::default()
            };
            field_inits.push(inner);
            continue;
        }

        // hangle things like Vec<u8> (brackets like <u8>)
        if let syn::Type::Path(b) = field_type {
            //eprintln!("{:?}", b);
            let last_segment = b.path.segments.last().unwrap();
            let last_segment_ident = &last_segment.ident;
            if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                let a = args.to_token_stream();
                let inner = quote! {
                    #field_ident: #last_segment_ident::#a::get_from_object(value, #field_name, &extended_types)?
                };
                field_inits.push(inner);
                continue;
            }
        }

        // regular types
        let inner = quote! {
            #field_ident: #field_type::get_from_object(value, #field_name, &extended_types)?
        };
        field_inits.push(inner);
    }

    let expanded = quote! {
        impl nskeyedunarchiver::de::Decodable for #struct_ident {
            fn is_type_of(classes: &[std::string::String]) -> bool {
                classes[0] == #struct_name
            }
            fn class(&self) -> &str { #struct_name }

            fn decode(value: nskeyedunarchiver::ValueRef, types: &[nskeyedunarchiver::de::ObjectType]) -> Result<Self, nskeyedunarchiver::DeError> {
                use nskeyedunarchiver::de::ObjectMember;
                let value = nskeyedunarchiver::as_object!(value)?;
                // One don't need to pass all types from the struct, only ones that don't appear there explicitly
                let mut extended_types = #object_types_macro;
                extended_types.extend_from_slice(types);
                Ok(
                    Self {
                        #(#field_inits),*
                    }
                )
            }
        }

        impl nskeyedunarchiver::de::ObjectMember for #struct_ident {
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
    };

    Ok(TokenStream::from(expanded))
}

#[proc_macro_derive(Decodable, attributes(decodable))]
pub fn decodable(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    decodable_impl(input)
        .unwrap_or_else(|e| e.to_compile_error().into())
}
