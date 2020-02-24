use crate::{Error, Result};
use mongodb::{options::ClientOptions, Client, Database};
use once_cell::sync::OnceCell;

static CLIENT: OnceCell<Client> = OnceCell::new();
static DATABASE: OnceCell<Database> = OnceCell::new();

pub fn client() -> Result<&'static Client> {
    match CLIENT.get() {
        Some(c) => Ok(c),
        None => Err(Error::NotConnected),
    }
}
pub fn database() -> Result<&'static Database> {
    match DATABASE.get() {
        Some(d) => Ok(d),
        None => Err(Error::NotConnected),
    }
}

pub fn connect(uri: &str, database: &str) -> Result<()> {
    connect_with_options(ClientOptions::parse(uri)?, database)
}
pub fn connect_with_options(options: ClientOptions, database: &str) -> Result<()> {
    let client = Client::with_options(options)?;
    if CLIENT.set(client).is_err() {
        return Err(Error::AlreadyConnected);
    }

    let database = CLIENT.get().unwrap().database(database);
    DATABASE.set(database).unwrap();

    Ok(())
}
