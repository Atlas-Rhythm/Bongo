use bongo::{BlockingModel, Model};
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(BlockingModel, Serialize, Deserialize)]
struct ExampleBlockingModel {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    age: u8,
}

#[derive(Model, Serialize, Deserialize)]
#[bongo(collection = "asyncModels")]
struct ExampleModel {
    _id: u64,
    contents: Vec<String>,
    author: ObjectId,
}
