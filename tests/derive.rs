use bongo::{BlockingModel, Model};
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(BlockingModel, Serialize, Deserialize)]
struct ExampleBlockingModel {
    id: ObjectId,
    name: String,
    age: u8,
}

#[derive(Model, Serialize, Deserialize)]
struct ExampleModel {
    id: ObjectId,
    name: String,
    age: u8,
}
