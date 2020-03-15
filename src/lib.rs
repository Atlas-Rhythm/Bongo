mod error;
mod globals;
#[doc(hidden)]
pub mod re_exports;

#[cfg(feature = "derive")]
pub use bongo_derive::*;

pub use crate::{error::Error, globals::*};
use bson::{bson, doc, Bson, Document};
use mongodb::{
    options::{ReplaceOptions, UpdateModifications},
    results::*,
    Collection,
};
use serde::{de::DeserializeOwned, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

pub trait BlockingModel: DeserializeOwned + Serialize {
    #[cfg(not(feature = "tokio"))]
    type Id: Into<Bson> + Clone;
    #[cfg(feature = "tokio")]
    type Id: Into<Bson> + Clone + Send;

    fn collection() -> Result<&'static Collection>;

    fn id(&self) -> Self::Id;
    fn id_query(&self) -> Document {
        doc! {"_id": self.id().into()}
    }

    fn estimated_document_count_sync() -> Result<i64> {
        Ok(Self::collection()?.estimated_document_count(None)?)
    }
    fn count_documents_sync<F>(filter: F) -> Result<i64>
    where
        F: Into<Option<Document>>,
    {
        Ok(Self::collection()?.count_documents(filter, None)?)
    }

    fn find_sync<F>(filter: F) -> Result<Vec<Self>>
    where
        F: Into<Option<Document>>,
    {
        Self::collection()?
            .find(filter, None)?
            .map(|r| match r {
                Ok(d) => bson::from_bson(d.into()).map_err(|e| e.into()),
                Err(e) => Err(e.into()),
            })
            .collect()
    }
    fn find_one_sync<F>(filter: F) -> Result<Option<Self>>
    where
        F: Into<Option<Document>>,
    {
        Ok(Self::collection()?
            .find_one(filter, None)?
            .map(|v| bson::from_bson(v.into()))
            .transpose()?)
    }
    fn find_by_id_sync(id: Self::Id) -> Result<Option<Self>> {
        Self::find_one_sync(doc! {"_id": id.into()})
    }

    fn insert_many_sync(docs: &[Self]) -> Result<InsertManyResult> {
        Ok(Self::collection()?.insert_many(to_documents(docs)?, None)?)
    }
    fn update_many_sync<Q, U>(query: Q, update: U) -> Result<UpdateResult>
    where
        Q: Into<Document>,
        U: Into<UpdateModifications>,
    {
        Ok(Self::collection()?.update_many(query.into(), update.into(), None)?)
    }
    fn delete_many_sync<Q>(query: Q) -> Result<DeleteResult>
    where
        Q: Into<Document>,
    {
        Ok(Self::collection()?.delete_many(query.into(), None)?)
    }

    fn save_sync(&self) -> Result<UpdateResult> {
        Ok(Self::collection()?.replace_one(
            self.id_query(),
            to_document(&self)?,
            ReplaceOptions {
                bypass_document_validation: None,
                upsert: Some(true),
                collation: None,
                hint: None,
                write_concern: None,
            },
        )?)
    }
    fn remove_sync(&self) -> Result<DeleteResult> {
        Ok(Self::collection()?.delete_one(self.id_query(), None)?)
    }
}

#[cfg(feature = "tokio")]
use async_trait::async_trait;
#[cfg(feature = "tokio")]
use tokio::task::spawn_blocking;

#[cfg(feature = "tokio")]
#[cfg_attr(feature = "tokio", async_trait)]
pub trait Model: BlockingModel + Send + Sync + 'static {
    async fn estimated_document_count() -> Result<i64> {
        spawn_blocking(Self::estimated_document_count_sync).await?
    }
    async fn count_documents<F>(filter: F) -> Result<i64>
    where
        F: Into<Option<Document>> + Send + 'static,
    {
        spawn_blocking(move || Self::count_documents_sync(filter)).await?
    }

    async fn find<F>(filter: F) -> Result<Vec<Self>>
    where
        F: Into<Option<Document>> + Send + 'static,
    {
        spawn_blocking(move || Self::find_sync(filter)).await?
    }
    async fn find_one<F>(filter: F) -> Result<Option<Self>>
    where
        F: Into<Option<Document>> + Send + 'static,
    {
        spawn_blocking(move || Self::find_one_sync(filter)).await?
    }
    async fn find_by_id(id: Self::Id) -> Result<Option<Self>> {
        spawn_blocking(move || Self::find_by_id_sync(id)).await?
    }

    async fn insert_many(docs: &[Self]) -> Result<InsertManyResult> {
        let docs = to_documents(docs)?;
        spawn_blocking(move || Ok(Self::collection()?.insert_many(docs, None)?)).await?
    }
    async fn update_many<Q, U>(query: Q, update: U) -> Result<UpdateResult>
    where
        Q: Into<Document> + Send + 'static,
        U: Into<UpdateModifications> + Send + 'static,
    {
        spawn_blocking(move || Ok(Self::update_many_sync(query.into(), update.into())?)).await?
    }
    async fn delete_many<Q>(query: Q) -> Result<DeleteResult>
    where
        Q: Into<Document> + Send + 'static,
    {
        spawn_blocking(move || Self::delete_many_sync(query)).await?
    }

    async fn save(&self) -> Result<UpdateResult> {
        let query = self.id_query();
        let replacement = to_document(self)?;
        spawn_blocking(move || {
            Ok(Self::collection()?.replace_one(
                query,
                replacement,
                ReplaceOptions {
                    bypass_document_validation: None,
                    upsert: Some(true),
                    collation: None,
                    hint: None,
                    write_concern: None,
                },
            )?)
        })
        .await?
    }
    async fn remove(&self) -> Result<DeleteResult> {
        let query = self.id_query();
        spawn_blocking(move || Ok(Self::collection()?.delete_one(query, None)?)).await?
    }
}

fn to_documents<T: Serialize>(docs: &[T]) -> Result<Vec<Document>> {
    docs.iter()
        .map(|s| match bson::to_bson(s) {
            Ok(b) => match b {
                Bson::Document(d) => Ok(d),
                _ => unreachable!(),
            },
            Err(e) => Err(e.into()),
        })
        .collect()
}
fn to_document<T: Serialize>(m: &T) -> Result<Document> {
    match bson::to_bson(m)? {
        Bson::Document(d) => Ok(d),
        _ => unreachable!(),
    }
}
