use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct NodeANR {
    pub ip4: String,
    pub pk: Vec<u8>,        // Public key as raw bytes
    pub pop: Vec<u8>,       // Proof-of-possession as bytes
    pub port: u16,
    pub signature: Option<Vec<u8>>, // Signature as bytes
    pub ts: u64,
    pub version: String,
    pub handshaked: bool,
    pub error: Option<String>,
    pub error_tries: u32,
    pub next_check: u64,
    pub has_chain_pop: bool,
}

// Simplified "database"
pub struct NodeANRStore {
    nodes: HashMap<Vec<u8>, NodeANR>, // Use pk bytes as key
}

impl NodeANRStore {
    pub fn new() -> Self {
        NodeANRStore {
            nodes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, anr: NodeANR) {
        let key = anr.pk.clone();
        if let Some(old) = self.nodes.get(&key) {
            if anr.ts <= old.ts {
                return; // ignore old record
            }
        }
        self.nodes.insert(key, anr);
    }

    pub fn get(&self, pk: &Vec<u8>) -> Option<&NodeANR> {
        self.nodes.get(pk)
    }

    pub fn handshaked_nodes(&self) -> Vec<&NodeANR> {
        self.nodes.values().filter(|n| n.handshaked).collect()
    }

    pub fn random_verified(&self, count: usize) -> Vec<&NodeANR> {
        let mut verified: Vec<_> = self.handshaked_nodes();
        verified.shuffle(&mut rand::thread_rng());
        verified.into_iter().take(count).collect()
    }
}

// Build NodeANR record
impl NodeANR {
    pub fn build(
        pk: Vec<u8>,
        pop: Vec<u8>,
        ip4: String,
        version: String,
        signature: Option<Vec<u8>>,
    ) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        NodeANR {
            ip4,
            pk,
            pop,
            port: 36969,
            signature,
            ts,
            version,
            handshaked: false,
            error: None,
            error_tries: 0,
            next_check: ts + 3,
            has_chain_pop: false,
        }
    }

    pub fn verify_signature(&self) -> bool {
        // Placeholder: implement your own verification logic on raw bytes
        self.signature.is_some()
    }
}
