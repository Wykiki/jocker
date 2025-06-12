use std::{thread::sleep, time::Duration};

use common::log;

fn main() {
    loop {
        log(&"To the WAR !!!".to_string());
        sleep(Duration::from_millis(10000));
    }
}
