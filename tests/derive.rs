use bongo::{BlockingModel, Model};
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize)]
struct User {
    #[serde(rename = "_id")]
    id: ObjectId,
    username: String,
    password: String,
}

#[derive(Model, Serialize, Deserialize)]
struct Todo {
    _id: i32,
    #[bongo(has_one(User))]
    author: ObjectId,
}

#[derive(Model, Serialize, Deserialize)]
struct Useless {
    #[serde(rename = "_id")]
    id: f64,
    #[bongo(has_many(BlockingUseless, "stuff", "stuff_async"))]
    stuff: Vec<String>,
}

#[derive(BlockingModel, Serialize, Deserialize)]
struct BlockingUseless {
    _id: String,
}
