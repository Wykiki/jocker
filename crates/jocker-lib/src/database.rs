use std::sync::{Arc, Mutex};

use rusqlite::Connection;

pub(crate) type Database = Arc<Mutex<Connection>>;
