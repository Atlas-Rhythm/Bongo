mod error;
mod globals;
#[doc(hidden)]
pub mod re_exports;

#[cfg(feature = "derive")]
pub use bongo_derive::*;

pub use crate::{error::Error, globals::*};
use bson::{bson, doc, oid::ObjectId, Bson, Document};
use mongodb::{options::ReplaceOptions, results::*, Collection};
use serde::{de::DeserializeOwned, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

pub trait BlockingModel: DeserializeOwned + Serialize {
    #[cfg(not(feature = "tokio"))]
    type Update: Serialize;
    #[cfg(feature = "tokio")]
    type Update: Serialize + Sync;

    fn collection() -> Result<&'static Collection>;

    fn id(&self) -> ObjectId;

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
    fn find_by_id_sync(id: ObjectId) -> Result<Option<Self>> {
        Self::find_one_sync(doc! {"_id": id})
    }

    fn insert_many_sync(docs: &[Self]) -> Result<InsertManyResult> {
        Ok(Self::collection()?.insert_many(iter_to_bson(docs)?, None)?)
    }
    fn update_many_sync<Q>(query: Q, update: &Self::Update) -> Result<UpdateResult>
    where
        Q: Into<Document>,
    {
        Ok(Self::collection()?.update_many(query.into(), to_document(update)?, None)?)
    }
    fn delete_many_sync<Q>(query: Q) -> Result<DeleteResult>
    where
        Q: Into<Document>,
    {
        Ok(Self::collection()?.delete_many(query.into(), None)?)
    }

    fn save_sync(&self) -> Result<UpdateResult> {
        Ok(Self::collection()?.replace_one(
            doc! {"_id": self.id()},
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
        Ok(Self::collection()?.delete_one(doc! {"_id": self.id()}, None)?)
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
    async fn find_by_id(id: ObjectId) -> Result<Option<Self>> {
        spawn_blocking(move || Self::find_by_id_sync(id)).await?
    }

    async fn insert_many(docs: &[Self]) -> Result<InsertManyResult> {
        let docs = iter_to_bson(docs)?;
        spawn_blocking(move || {
            Self::collection()?
                .insert_many(docs, None)
                .map_err(|e| Error::from(e))
        })
        .await?
    }
    async fn update_many<Q>(query: Q, update: &Self::Update) -> Result<UpdateResult>
    where
        Q: Into<Document> + Send + 'static,
    {
        let update = to_document(update)?;
        spawn_blocking(move || {
            Self::collection()?
                .update_many(query.into(), update, None)
                .map_err(|e| Error::from(e))
        })
        .await?
    }
    async fn delete_many<Q>(query: Q) -> Result<DeleteResult>
    where
        Q: Into<Document> + Send + 'static,
    {
        spawn_blocking(move || Self::delete_many_sync(query)).await?
    }

    async fn save(&self) -> Result<UpdateResult> {
        let query = doc! {"_id": self.id()};
        let replacement = to_document(self)?;
        spawn_blocking(move || {
            Self::collection()?
                .replace_one(
                    query,
                    replacement,
                    ReplaceOptions {
                        bypass_document_validation: None,
                        upsert: Some(true),
                        collation: None,
                        hint: None,
                        write_concern: None,
                    },
                )
                .map_err(|e| Error::from(e))
        })
        .await?
    }
    async fn remove(&self) -> Result<DeleteResult> {
        let query = doc! {"_id": self.id()};
        spawn_blocking(move || {
            Self::collection()?
                .delete_one(query, None)
                .map_err(|e| Error::from(e))
        })
        .await?
    }
}

fn iter_to_bson<T: Serialize>(docs: &[T]) -> Result<Vec<Document>> {
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
