use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, parse_macro_input};
use syn::{Error, Result, spanned::Spanned};

// All possible attributes
// #[decodable(rename = "foo")], #[decodable(skip)]
const BOOL_ATTRS: [&str; 3] = ["skip", "unhandled", "default"];
const STR_ATTRS: [&str; 1] = ["rename"];

/// Parses all attributes that come from #[decodable(...)]
#[derive(Debug, Default)]
struct MacroAttributes {
    str_attrs: HashMap<String, String>,
    bool_attrs: Vec<String>,
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
            return Err(Error::new(
                decodable_attr.path().span(),
                "Unable to parse attributes",
            ));
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
                    return Err(Error::new(
                        decodable_attr.path().span(),
                        "An attribute cannot be set more than once",
                    ));
                }
                if !BOOL_ATTRS.contains(&pair[0]) {
                    return Err(Error::new(
                        decodable_attr.path().span(),
                        format!("Unknown attribute `{}`", pair[0]),
                    ));
                }
                bool_attrs.push(pair[0].to_string());
                continue;
            }

            // Handle cases like `rename = = "smh"`
            if pair.len() != 2 {
                return Err(Error::new(
                    decodable_attr.path().span(),
                    "Incorrect attribute",
                ));
            }

            let attr_name = pair[0];
            let attr_value = pair[1];
            if !attr_value.starts_with("\"") || !attr_value.ends_with("\"") {
                return Err(Error::new(
                    decodable_attr.path().span(),
                    "An attribute value should start and end with a quote",
                ));
            }
            let attr_value = &attr_value[1..attr_value.len() - 1]; // remove quotes ""
            if str_attrs.contains_key(pair[0]) {
                return Err(Error::new(
                    decodable_attr.path().span(),
                    "An attribute cannot be set more than once",
                ));
            }
            if !STR_ATTRS.contains(&pair[0]) {
                return Err(Error::new(
                    decodable_attr.path().span(),
                    format!("Unknown attribute `{}`", pair[0]),
                ));
            }
            str_attrs.insert(attr_name.to_string(), attr_value.to_string());
        }

        if (bool_attrs.contains(&"skip".to_string())
            || bool_attrs.contains(&"unhandled".to_string()))
            && (!str_attrs.is_empty() || bool_attrs.len() > 1)
        {
            return Err(Error::new(
                decodable_attr.path().span(),
                "`skip` cannot be used with other arguments",
            ));
        }

        Ok(Self {
            str_attrs,
            bool_attrs,
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
            cur_struct.fields.span(),
            "Only structs with named fields are supported",
        ));
    };

    let struct_ident = &input.ident;
    let mut struct_name = struct_ident.to_string();

    let struct_attrs = MacroAttributes::try_from(input.attrs.as_slice())?;
    if let Some(new_name) = struct_attrs.str_attrs.get("rename") {
        struct_name = new_name.to_string();
    }

    if struct_attrs.bool_attrs.contains(&"skip".to_string())
        || struct_attrs.bool_attrs.contains(&"unhandled".to_string())
        || struct_attrs.bool_attrs.contains(&"default".to_string())
    {
        return Err(Error::new(
            input.attrs[0].path().span(),
            "`skip`, `unhandled`, `default` can only be used for fields",
        ));
    }

    let mut field_inits: Vec<proc_macro2::TokenStream> =
        Vec::with_capacity(named_fields.named.len());

    // First interator over fields to collect all field names
    let mut field_names = Vec::with_capacity(named_fields.named.len());
    for f in &named_fields.named {
        // hangle things like Vec<u8> (brackets like <u8>)
        let field_attrs = MacroAttributes::try_from(f.attrs.as_slice())?;
        if field_attrs.bool_attrs.contains(&"skip".to_string())
            || field_attrs.bool_attrs.contains(&"unhandled".to_string())
        {
            continue;
        }

        // For `unhandled`
        let mut field_name = f.ident.as_ref().unwrap().to_string();
        if let Some(new_name) = field_attrs.str_attrs.get("rename") {
            field_name = new_name.to_string();
        }
        field_names.push(quote!(#field_name));
    }

    // Second iterator over fields. Now we build field initializators:
    // fieldName: Type::decode(value)
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

        // #[decodable(unhandled)]
        // Find all unhandled fields and create HashMap<String, ValueRef>
        // of them and their values
        if field_attrs.bool_attrs.contains(&"unhandled".to_string()) {
            let inner = quote! {
                #field_ident: {
                    let mut unhandled_fields = vec![];
                    let keys = value.keys();
                    let fields = vec![#(#field_names),*];
                    for key in keys {
                        if !fields.contains(&key.as_str()) {
                            unhandled_fields.push(key);
                        }
                    }

                    let mut unhandled = std::collections::HashMap::with_capacity(unhandled_fields.len());
                    for field in unhandled_fields {
                        let Some(value) = value.as_map().get(field) else {
                            continue;
                        };

                        unhandled.insert(
                            field.to_string(),
                            value.clone(),
                        );
                    }

                    unhandled
                }
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
                let mut inner = quote! {
                    #field_ident: {
                        let v = value
                            .as_map()
                            .get(#field_name)
                            .ok_or(nskeyedunarchiver::DeError::MissingObjectKey(value.class().into(), #field_name.into()))?;
                        #last_segment_ident::#a::decode(v)?
                    }
                };

                // This is hacky, it panics if there's a custom defined struct/enum
                // with the same `Option` name
                // May be replaced with TypeId::of::<std::option::Option<T>>() I guess...
                let mut is_option = false;
                if last_segment_ident.to_string().trim() == "Option" {
                    is_option = true;
                }

                // Handle #[decodable(default)] and Option<T>
                // Default::default() for Option is None
                if field_attrs.bool_attrs.contains(&"default".to_string()) || is_option {
                    inner = quote! {
                        #field_ident: {
                            if let Some(v) = value.as_map().get(#field_name) {
                                #last_segment_ident::#a::decode(v)?
                            }
                            else {
                                Default::default()
                            }
                        }
                    };
                }
                field_inits.push(inner);
                continue;
            }
        }

        // regular types
        let mut inner = quote! {
            #field_ident: {
                let v = value.as_map().get(#field_name)
                .ok_or(nskeyedunarchiver::DeError::MissingObjectKey(value.class().into(), #field_name.into()))?;
                #field_type::decode(v)?
            }
        };
        // Handle #[decodable(default)]
        if field_attrs.bool_attrs.contains(&"default".to_string()) {
            inner = quote! {
                #field_ident: {
                    if let Some(v) = value.as_map().get(#field_name) {
                        #field_type::decode(v)?
                    }
                    else {
                        Default::default()
                    }
                }
            };
        }
        field_inits.push(inner);
    }

    let expanded = quote! {
        impl nskeyedunarchiver::de::Decodable for #struct_ident {
            fn decode(value: &nskeyedunarchiver::ObjectValue) -> Result<Self, nskeyedunarchiver::DeError> {
                use nskeyedunarchiver::de::Decodable;
                let nskeyedunarchiver::ObjectValue::Ref(value) = value else {
                    return Err(nskeyedunarchiver::DeError::ExpectedObject);
                };
                let value = value.as_object().ok_or(nskeyedunarchiver::DeError::ExpectedObject)?;
                if #struct_name != value.class() {
                    return Err(nskeyedunarchiver::DeError::Message(
                        format!("Expected {} class, found {}", #struct_name, value.class())
                    ).into());
                }
                Ok(
                    Self {
                        #(#field_inits),*
                    }
                )
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
    if !enum_attrs.bool_attrs.is_empty() || !enum_attrs.str_attrs.is_empty() {
        return Err(Error::new(
            input.span(),
            "Attributes for enums are not supported",
        ));
    }

    let variants = &cur_enum.variants;
    let mut variants_inits: Vec<proc_macro2::TokenStream> = Vec::with_capacity(variants.len());

    // First interator over variants. We find all their types to build a Vec<ObjectType>
    // to pass it to `decode` methods
    let mut object_types = Vec::with_capacity(variants.len());
    for v in variants {
        // hangle things like Vec<u8> (brackets like <u8>)
        let field_attrs = MacroAttributes::try_from(v.attrs.as_slice())?;
        if field_attrs.bool_attrs.contains(&"skip".to_string()) {
            continue;
        }
        if !field_attrs.str_attrs.is_empty() {
            return Err(Error::new(
                v.attrs[0].path().span(),
                "Only `skip` attribute is valid for enum variants",
            ));
        }

        if v.fields.len() != 1 {
            return Err(Error::new(
                v.fields.span(),
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

        if !field_attrs.str_attrs.is_empty() {
            return Err(Error::new(
                v.attrs[0].path().span(),
                "This attribute is not supported for enum variants",
            ));
        }
        if field_attrs.bool_attrs.contains(&"skip".to_string()) {
            continue;
        }

        // hangle things like Vec<u8> (brackets like <u8>)
        if let syn::Type::Path(b) = field_type {
            let last_segment = b.path.segments.last().unwrap();
            let last_segment_ident = &last_segment.ident;
            if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                let a = args.to_token_stream();
                let inner = quote! {
                    if let Ok(v) = #last_segment_ident::#a::decode(value) {
                        return Ok(Self::#field_ident(v));
                    }
                };
                variants_inits.push(inner);
                continue;
            }
        }

        // regular types
        let inner = quote! {
            if let Ok(v) = #field_type::decode(value) {
                return Ok(Self::#field_ident(v));
            }
        };
        variants_inits.push(inner);
    }

    let expanded = quote! {
        impl nskeyedunarchiver::de::Decodable for #enum_ident {
            fn decode(value: &nskeyedunarchiver::ObjectValue) -> Result<Self, nskeyedunarchiver::DeError>
            where
                Self: Sized {
                #(#variants_inits)*

                Err(nskeyedunarchiver::DeError::Message(format!(
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
            input.ident.span(),
            "Only structs and enums are supported",
        )),
    }
}

/// Derive macro generating an impl of the trait `Decodable`.
#[proc_macro_derive(Decodable, attributes(decodable))]
pub fn decodable(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    decodable_impl(input).unwrap_or_else(|e| e.to_compile_error().into())
}
