use std::{thread::sleep, time::Duration};

use common::log;
use rand::{thread_rng, Rng};

fn main() {
    let sayings = ["Peace", "Harmony", "Serenity"];
    let mut rng = thread_rng();
    loop {
        let say = sayings[rng.gen_range(0..sayings.len())];
        log(&say);
        sleep(Duration::from_millis(5000));
    }
}
