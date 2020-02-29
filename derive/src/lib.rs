extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, Ident};

#[proc_macro_derive(BlockingModel)]
pub fn blocking_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(blocking_model_impl(input))
}

#[proc_macro_derive(Model)]
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
    let ident = input.ident;

    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(f) => f,
            _ => panic!("bongo only supports named fields"),
        },
        _ => panic!("bongo only supports structs"),
    };

    let update_ident = format_ident!("{}Update", ident);
    let update_def = update_struct(&update_ident, &fields);

    let collection = camel_case_ident(&ident.to_string());

    quote! {
        #update_def

        impl ::bongo::BlockingModel for #ident {
            type Update = #update_ident;

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

            fn id(&self) -> ::bongo::re_exports::bson::oid::ObjectId {
                self.id.clone()
            }
            fn update(&self) -> Option<&Self::Update> {
                None
            }
        }
    }
}

fn update_struct(ident: &Ident, fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let idents = fields.named.iter().map(|f| &f.ident);
    let types = fields.named.iter().map(|f| &f.ty);
    quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        pub struct #ident {
            #(pub #idents: Option<#types>,)*
        }
    }
}

fn camel_case_ident(s: &str) -> String {
    let first_char = s.chars().next().unwrap().to_lowercase().next().unwrap();
    format!("{}{}", first_char, &s[1..])
}
