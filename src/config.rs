use std::{collections::HashMap, sync::RwLock};

use tokio::sync::OnceCell;

static KV: OnceCell<RwLock<HashMap<&str, String>>> = OnceCell::const_new();
const DIR_KEY: &str = "dir";
const DBFILENAME_KEY: &str = "dbfilename";

pub fn init() {
    KV.set(RwLock::new(HashMap::new()))
        .expect("KV should be set only once");
}

pub fn set_dir(dir: &str) {
    KV.get()
        .expect("KV should be initialized")
        .write()
        .unwrap()
        .insert(DIR_KEY, dir.to_string());
}

pub fn get_dir() -> String {
    KV.get()
        .expect("KV should be initialized")
        .read()
        .unwrap()
        .get(DIR_KEY)
        .expect("dir should be set")
        .to_string()
}

pub fn set_dbfilename(filename: &str) {
    KV.get()
        .expect("KV should be initialized")
        .write()
        .unwrap()
        .insert(DBFILENAME_KEY, filename.to_string());
}

pub fn get_dbfilename() -> String {
    KV.get()
        .expect("KV should be initialized")
        .read()
        .unwrap()
        .get(DBFILENAME_KEY)
        .expect("dbfilename should be set")
        .to_string()
}
