use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

// Simulate persistent_term
pub static PERSISTENT_TERM: Lazy<Mutex<HashMap<&'static str, AtomicI32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
