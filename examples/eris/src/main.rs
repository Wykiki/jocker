use std::{thread::sleep, time::Duration};

use common::log;
use rand::{thread_rng, Rng};

fn main() {
    let childlings = [
        "Dysnomia",
        "Ponos",
        "Atë",
        "Lethe",
        "Limos",
        "Algos",
        "Hysminai",
        "Hakai",
        "Phonoi",
        "Androktasiai",
        "Neikea",
        "Amphilogiai",
        "Horkos",
        "Pseudea",
        "Logoi",
    ];
    let mut rng = thread_rng();
    loop {
        let say = childlings[rng.gen_range(0..childlings.len())];
        log(&say);
        sleep(Duration::from_millis(rng.gen_range(500..5500)));
    }
}
