extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Field, Fields, FieldsNamed, Ident, Lit, Meta,
    MetaList, NestedMeta, Path,
};

#[proc_macro_derive(BlockingModel, attributes(bongo))]
pub fn blocking_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(blocking_model_impl(input).0)
}

#[proc_macro_derive(Model, attributes(bongo))]
pub fn model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident.clone();

    let (blocking_impl, relations) = blocking_model_impl(input);
    let Relations {
        getters, checks, ..
    } = relations;

    let expanded = quote! {
        #blocking_impl

        #[::bongo::re_exports::async_trait::async_trait]
        impl ::bongo::Model for #ident {
            async fn check_relations(&self) -> ::bongo::Result<()> {
                use ::bongo::{re_exports::tokio::task, BlockingModel, Error};
                use ::bson::{bson, doc};

                #(#checks)*
                Ok(())
            }
        }

        impl #ident {
            #(#getters)*
        }
    };
    TokenStream::from(expanded)
}

fn blocking_model_impl(input: DeriveInput) -> (proc_macro2::TokenStream, Relations) {
    let ident = &input.ident;
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => f,
            _ => panic!("bongo only supports named fields"),
        },
        _ => panic!("bongo only supports structs"),
    };

    let collection = collection_name(&input);

    let id = id_field(fields);
    let id_ty = &id.ty;
    let id_ident = id.ident.as_ref().unwrap();

    let relations = relations(fields);
    let Relations {
        getters_sync,
        checks_sync,
        ..
    } = &relations;

    (
        quote! {
            impl ::bongo::BlockingModel for #ident {
                type Id = #id_ty;

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
                    self.#id_ident.clone()
                }

                fn check_relations_sync(&self) -> ::bongo::Result<()> {
                    use ::bongo::{BlockingModel, Error};
                    use ::bson::{bson, doc};

                    #(#checks_sync)*
                    Ok(())
                }
            }

            impl #ident {
                #(#getters_sync)*
            }
        },
        relations,
    )
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

struct Relations {
    getters_sync: Vec<proc_macro2::TokenStream>,
    getters: Vec<proc_macro2::TokenStream>,
    checks_sync: Vec<proc_macro2::TokenStream>,
    checks: Vec<proc_macro2::TokenStream>,
}

fn relations(fields: &FieldsNamed) -> Relations {
    let mut getters_sync = Vec::new();
    let mut getters = Vec::new();
    let mut checks_sync = Vec::new();
    let mut checks = Vec::new();

    for field in &fields.named {
        let ident = field.ident.as_ref().unwrap();

        let attrs = &field.attrs;
        for attr in attrs {
            if !attr_is_bongo(attr) {
                continue;
            }

            let attr = parse_attr(attr);
            for opt in attr.nested {
                let ml = match opt {
                    NestedMeta::Meta(Meta::List(ml)) => ml,
                    _ => continue,
                };

                let relation = if ml.path.is_ident("has_one") {
                    one_relation(&ml, ident)
                } else if ml.path.is_ident("has_many") {
                    many_relation(&ml, ident)
                } else {
                    continue;
                };
                getters_sync.push(relation.getter_sync);
                getters.push(relation.getter);
                checks_sync.push(relation.check_sync);
                checks.push(relation.check);
            }
        }
    }

    Relations {
        getters_sync,
        getters,
        checks_sync,
        checks,
    }
}

struct Relation {
    getter_sync: proc_macro2::TokenStream,
    getter: proc_macro2::TokenStream,
    check_sync: proc_macro2::TokenStream,
    check: proc_macro2::TokenStream,
}

fn one_relation(ml: &MetaList, ident: &Ident) -> Relation {
    let rel = relation_info(ml, ident);
    let RelationInfo {
        model,
        sync_getter_name,
        getter_name,
    } = rel;

    let getter_sync = quote! {
        pub fn #sync_getter_name(&self) -> ::bongo::Result<#model> {
            use ::bongo::{BlockingModel, Error};

            match #model::find_by_id_sync(self.#ident.clone())? {
                Some(m) => Ok(m),
                None => Err(Error::Relation(format!(
                    "referenced document with id {} doesn't exist",
                    self.#ident,
                ))),
            }
        }
    };
    let getter = quote! {
        pub async fn #getter_name(&self) -> ::bongo::Result<#model> {
            use ::bongo::{re_exports::tokio::task, BlockingModel, Error};

            let id = self.#ident.clone();
            match task::spawn_blocking(move || #model::find_by_id_sync(id)).await?? {
                Some(m) => Ok(m),
                None => Err(Error::Relation(format!(
                    "referenced document with id {} doesn't exist",
                    self.#ident,
                ))),
            }
        }
    };
    let check_sync = quote! {
        if #model::count_documents_sync(doc! {"_id": self.#ident.clone()})? < 1 {
            return Err(Error::Relation(format!(
                "referenced document with id {} doesn't exist",
                self.#ident,
            )));
        }
    };
    let check = quote! {
        let query = doc! {"_id": self.#ident.clone()};
        if task::spawn_blocking(move || #model::count_documents_sync(query)).await?? < 1 {
            return Err(Error::Relation(format!(
                "referenced document with id {} doesn't exist",
                self.#ident,
            )));
        }
    };

    Relation {
        getter_sync,
        getter,
        check_sync,
        check,
    }
}

fn many_relation(ml: &MetaList, ident: &Ident) -> Relation {
    let rel = relation_info(ml, ident);
    let RelationInfo {
        model,
        sync_getter_name,
        getter_name,
    } = rel;

    let getter_sync = quote! {
        pub fn #sync_getter_name(&self) -> ::bongo::Result<Vec<#model>> {
            use ::bongo::{BlockingModel, Error};

            let mut result = Vec::with_capacity(self.#ident.len());
            for id in &self.#ident {
                match #model::find_by_id_sync(id.clone())? {
                    Some(m) => result.push(m),
                    None => {
                        return Err(Error::Relation(format!(
                            "referenced document with id {} doesn't exist",
                            id,
                        )));
                    },
                }
            }
            Ok(result)
        }
    };
    let getter = quote! {
        pub async fn #getter_name(&self) -> ::bongo::Result<Vec<#model>> {
            use ::bongo::{re_exports::tokio::task, BlockingModel, Error};

            let mut result = Vec::with_capacity(self.#ident.len());
            for id in &self.#ident {
                let move_id = id.clone();
                match task::spawn_blocking(move || #model::find_by_id_sync(move_id)).await?? {
                    Some(m) => result.push(m),
                    None => {
                        return Err(Error::Relation(format!(
                            "referenced document with id {} doesn't exist",
                            id,
                        )));
                    },
                }
            }
            Ok(result)
        }
    };
    let check_sync = quote! {
        for id in &self.#ident {
            if #model::count_documents_sync(doc! {"_id": id.clone()})? < 1 {
                return Err(Error::Relation(format!(
                    "referenced document with id {} doesn't exist",
                    id,
                )));
            }
        }
    };
    let check = quote! {
        for id in &self.#ident {
            let query = doc! {"_id": id.clone()};
            if task::spawn_blocking(move || #model::count_documents_sync(query)).await?? < 1 {
                return Err(Error::Relation(format!(
                    "referenced document with id {} doesn't exist",
                    id,
                )));
            }
        }
    };

    Relation {
        getter_sync,
        getter,
        check_sync,
        check,
    }
}

struct RelationInfo<'a> {
    model: &'a Path,
    sync_getter_name: Ident,
    getter_name: Ident,
}

fn relation_info<'a, 'b>(ml: &'a MetaList, ident: &'b Ident) -> RelationInfo<'a> {
    let nested = &ml.nested;
    let mut nested_iter = nested.iter();

    let model = match nested_iter.next() {
        Some(NestedMeta::Meta(Meta::Path(p))) => p,
        _ => panic!("first argument of relation attribute must be the target type"),
    };
    let sync_getter_name = match nested_iter.next() {
        Some(NestedMeta::Lit(Lit::Str(s))) => format_ident!("{}", s.value()),
        None => format_ident!("{}_sync", ident),
        _ => panic!(
            "second argument of relation attribute must be the synchronous getter name as a string literal"
        ),
    };
    let getter_name = match nested_iter.next() {
        Some(NestedMeta::Lit(Lit::Str(s))) => format_ident!("{}", s.value()),
        None => format_ident!("{}", ident),
        _ => panic!(
            "second argument of relation attribute must be the getter name as a string literal"
        ),
    };

    RelationInfo {
        model,
        sync_getter_name,
        getter_name,
    }
}
