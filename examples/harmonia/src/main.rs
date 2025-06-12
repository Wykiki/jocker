use std::{thread::sleep, time::Duration};

use common::log;

fn main() {
    let sayings = ["Peace", "Harmony", "Serenity"];
    loop {
        let say = sayings[fastrand::usize(..sayings.len())];
        log(&say);
        sleep(Duration::from_millis(5000));
    }
}
