use std::fmt::Display;

use chrono::Utc;

pub fn log<T: Display>(content: &T) {
    let now = Utc::now().to_rfc3339();
    println!("{now} : {content}")
}
