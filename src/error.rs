use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("mongodb error: {0}")]
    MongoDb(#[from] mongodb::error::Error),
    #[error("bson decoding error: {0}")]
    BsonDecode(#[from] bson::DecoderError),
    #[error("bson decoding error: {0}")]
    BsonEncode(#[from] bson::EncoderError),
    #[error("tried to connect multiple times")]
    AlreadyConnected,
    #[error("tried to access unconnected client")]
    NotConnected,

    #[cfg(feature = "tokio")]
    #[cfg_attr(feature = "tokio", error("task error: {0}"))]
    Task(#[from] tokio::task::JoinError),
}
