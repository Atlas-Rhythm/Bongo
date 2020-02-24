use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("mongodb error")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("bson decoding error")]
    BsonDecode(#[from] bson::DecoderError),
    #[error("bson decoding error")]
    BsonEncode(#[from] bson::EncoderError),
    #[error("tried to connect multiple times")]
    AlreadyConnected,
    #[error("tried to access unconnected client")]
    NotConnected,

    #[cfg(feature = "tokio")]
    #[cfg_attr(feature = "tokio", error("task error"))]
    Task(#[from] tokio::task::JoinError),
}
