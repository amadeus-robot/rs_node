use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};

//  Active Node Record

pub struct NodeANR {
    pub ip4: Ipv4Addr,
    pub pk: String,
    pub pop: String,
    pub port: u16,
    pub signature: String,
    pub ts: u64,
    pub version: String,
}

impl NodeANR {
    
    pub fn set_handshaked () {
        
    }
}
