use std::time::{SystemTime, UNIX_EPOCH};

pub fn log(log: &str) {
    println!("{}: {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), log);
}
