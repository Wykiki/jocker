use std::fmt::Display;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn log<T: Display>(content: &T) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("{now} : {content}")
}
