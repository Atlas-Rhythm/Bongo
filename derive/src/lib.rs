extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Field, Fields, FieldsNamed, Lit, Meta,
    MetaList, NestedMeta,
};

#[proc_macro_derive(BlockingModel, attributes(bongo))]
pub fn blocking_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(blocking_model_impl(input))
}

#[proc_macro_derive(Model, attributes(bongo))]
pub fn model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident.clone();

    let blocking_impl = blocking_model_impl(input);

    let expanded = quote! {
        #blocking_impl

        impl ::bongo::Model for #ident {}
    };
    TokenStream::from(expanded)
}

fn blocking_model_impl(input: DeriveInput) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => f,
            _ => panic!("bongo only supports named fields"),
        },
        _ => panic!("bongo only supports structs"),
    };

    let collection = collection_name(&input);
    let id = &id_field(fields).ty;

    quote! {
        impl ::bongo::BlockingModel for #ident {
            type Id = #id;

            fn collection() -> ::bongo::Result<&'static ::bongo::re_exports::mongodb::Collection> {
                use ::bongo::re_exports::{
                    mongodb::Collection,
                    once_cell::sync::OnceCell,
                };

                static COLLECTION: OnceCell<Collection> = OnceCell::new();

                if let Some(c) = COLLECTION.get() {
                    return Ok(c);
                }

                COLLECTION.set(::bongo::database()?.collection(#collection)).unwrap();
                Ok(COLLECTION.get().unwrap())
            }

            fn id(&self) -> Self::Id {
                self.id.clone()
            }
        }
    }
}

fn attr_is_bongo(attr: &Attribute) -> bool {
    attr.path.is_ident("bongo")
}

fn parse_attr(attr: &Attribute) -> MetaList {
    match attr.parse_meta() {
        Ok(Meta::List(l)) => l,
        _ => panic!("invalid attribute syntax"),
    }
}

fn camel_case(s: &str) -> String {
    let first_char = s.as_bytes()[0].to_ascii_lowercase() as char;
    format!("{}{}s", first_char, &s[1..])
}

fn collection_name(input: &DeriveInput) -> String {
    let attrs = &input.attrs;
    let mut result = None;
    for attr in attrs {
        if !attr_is_bongo(attr) {
            continue;
        }

        let attr = parse_attr(attr);
        for opt in attr.nested {
            let nv = match opt {
                NestedMeta::Meta(Meta::NameValue(nv)) => nv,
                _ => continue,
            };
            if nv.path.is_ident("collection") {
                match nv.lit {
                    Lit::Str(s) => result = Some(s.value()),
                    _ => panic!("collection name should be a string literal"),
                }
            }
        }
    }
    result.unwrap_or_else(|| camel_case(&input.ident.to_string()))
}

fn id_field(fields: &FieldsNamed) -> &Field {
    for field in &fields.named {
        if &field.ident.as_ref().unwrap().to_string() == "_id" {
            return field;
        }
        for attr in &field.attrs {
            if !attr.path.is_ident("serde") {
                continue;
            }

            let attr = match attr.parse_meta() {
                Ok(Meta::List(l)) => l,
                _ => continue,
            };
            for opt in attr.nested {
                let nv = match opt {
                    NestedMeta::Meta(Meta::NameValue(nv)) => nv,
                    _ => continue,
                };
                if nv.path.is_ident("rename") {
                    match nv.lit {
                        Lit::Str(s) => {
                            if &s.value() == "_id" {
                                return field;
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }
    }
    panic!("no _id field on struct");
}
