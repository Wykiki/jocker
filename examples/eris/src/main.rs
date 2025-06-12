use std::{thread::sleep, time::Duration};

use common::log;

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
    loop {
        let say = childlings[fastrand::usize(0..childlings.len())];
        log(&say);
        sleep(Duration::from_millis(fastrand::u64(500..5500)));
    }
}
