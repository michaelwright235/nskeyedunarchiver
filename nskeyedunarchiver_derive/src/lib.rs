use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};
use syn::{spanned::Spanned, Error, Result};

// All possible attributes
// #[decodable(rename = "foo")], #[decodable(skip)]
const BOOL_ATTRS: [&str; 1] = ["skip"];
const STR_ATTRS: [&str; 1] = ["rename"];

/// Parses all attributes that come from #[decodable(...)]
#[derive(Debug, Default)]
struct MacroAttributes {
    str_attrs: HashMap<String, String>,
    bool_attrs: Vec<String>
}

impl TryFrom<&[syn::Attribute]> for MacroAttributes {
    type Error = syn::Error;

    fn try_from(value: &[syn::Attribute]) -> std::result::Result<Self, Self::Error> {
        // There may be other attributes, we find "decodable"
        let mut decodable_attr = None;
        for attr in value {
            if attr.path().get_ident().unwrap() == "decodable" {
                decodable_attr = Some(attr);
            }
        }

        // If we didn't find one, return an empty struct
        let Some(decodable_attr) = decodable_attr else {
                return Ok(Self::default());
        };

        let syn::Meta::List(list) = &decodable_attr.meta else {
            return Err(Error::new(decodable_attr.path().span(), "Unable to parse attributes"));
        };

        // Make a plain string out of an attribute contents
        let raw_attr = list.tokens.to_string();
        let mut str_attrs = HashMap::new();
        let mut bool_attrs = Vec::new();

        // Split it with a comma "," and iterate
        for param in raw_attr.split(",") {

            // Split a pair with "=" (name and contents)
            let mut pair: Vec<&str> = param.split("=").collect();
            for s in &mut pair {
                *s = s.trim();
            }

            // There may be bool attributes (like `skip`, without "="), so we handle it
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

            // Handle cases like `rename = = "smh"`
            if pair.len() != 2 {
                return Err(Error::new(decodable_attr.path().span(), "Incorrect attribute"));
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

// Implements Dedocable and ObjectMember for structs
fn decodable_struct(input: &DeriveInput) -> Result<TokenStream> {
    let syn::Data::Struct(cur_struct) = &input.data else {
        unreachable!()
    };
    let syn::Fields::Named(named_fields) = &cur_struct.fields else {
        return Err(Error::new(
            cur_struct.fields.span().clone(),
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

    // First interator over fields. We find all their types to build a Vec<ObjectType>
    // to pass it to `get_from_object` methods
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

    // Second iterator over fields. Now we build field initializators:
    // fieldName: Type::get_from_object(value, "field_name", &extended_types)
    // We put them all inside Self {...}
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

// Implements Dedocable for enums
fn decodable_enum(input: &DeriveInput) -> Result<TokenStream> {
    let syn::Data::Enum(cur_enum) = &input.data else {
        unreachable!()
    };
    let enum_ident = &input.ident;

    let enum_attrs = MacroAttributes::try_from(input.attrs.as_slice())?;
    if enum_attrs.bool_attrs.len() != 0 || enum_attrs.str_attrs.len() != 0 {
        return Err(Error::new(
            input.span(),
            "Attributes for enums are not supported",
        ));
    }

    let variants = &cur_enum.variants;
    let mut variants_inits: Vec<proc_macro2::TokenStream> =
    Vec::with_capacity(variants.len());

    // First interator over variants. We find all their types to build a Vec<ObjectType>
    // to pass it to `decode` methods
    let mut object_types = Vec::with_capacity(variants.len());
    for v in variants {
        // hangle things like Vec<u8> (brackets like <u8>)
        let field_attrs = MacroAttributes::try_from(v.attrs.as_slice())?;
        if field_attrs.bool_attrs.contains(&"skip".to_string()) {
            continue;
        }
        if field_attrs.str_attrs.len() != 0 {
            return Err(Error::new(
                v.attrs[0].path().span().clone(),
                "Only `skip` attribute is valid for enum variants",
            ));
        }

        if *&v.fields.len() != 1 {
            return Err(Error::new(
                v.fields.span().clone(),
                "An enum variant can only have one field",
            ));
        }
        let field = &v.fields.iter().next().unwrap();

        if let syn::Type::Path(b) = &field.ty {
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
        object_types.push(field.ty.to_token_stream());
    }

    // Second iterator over variant. We build if statements that check if
    // an underlying object is decodable into a type of each variant.
    // if let Ok(v) = Type::decode(value.clone(), types) {
    //    return Ok(Self::Variant(v));
    // }
    for v in variants {
        let field_ident = &v.ident;
        let field_type = &v.fields.iter().next().unwrap().ty;

        let field_attrs = MacroAttributes::try_from(v.attrs.as_slice())?;

        if field_attrs.str_attrs.len() != 0 {
            return Err(Error::new(
                v.attrs[0].path().span().clone(),
                "This attribute is not supported for enum variants",
            ));
        }
        if field_attrs.bool_attrs.contains(&"skip".to_string()) {
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
                    if let Ok(v) = #last_segment_ident::#a::decode(value.clone(), types) {
                        return Ok(Self::#field_ident(v));
                    }
                };
                variants_inits.push(inner);
                continue;
            }
        }

        // regular types
        let inner = quote! {
            if let Ok(v) = #field_type::decode(value.clone(), types) {
                return Ok(Self::#field_ident(v));
            }
        };
        variants_inits.push(inner);
    }

    let expanded = quote! {
        impl Decodable for #enum_ident {
            fn is_type_of(classes: &[String]) -> bool
            where
                Self: Sized {
                false
            }

            fn class(&self) -> &str {
                ""
            }

            fn decode(value: nskeyedunarchiver::ValueRef, types: &[ObjectType]) -> Result<Self, DeError>
            where
                Self: Sized {
                #(#variants_inits)*

                Err(DeError::Message(format!(
                    "Undecodable object for enum: {value:?}",
                )))
            }
        }
    };

    Ok(TokenStream::from(expanded))
}

fn decodable_impl(input: DeriveInput) -> Result<TokenStream> {
    match &input.data {
        syn::Data::Struct(_) => decodable_struct(&input),
        syn::Data::Enum(_) => decodable_enum(&input),
        _ => Err(Error::new(
            input.ident.span().clone(),
            "Only structs and enums are supported",
        ))
    }
}

/// Derive macro generating an impl of the trait `Decodable`.
#[proc_macro_derive(Decodable, attributes(decodable))]
pub fn decodable(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    decodable_impl(input)
        .unwrap_or_else(|e| e.to_compile_error().into())
}
