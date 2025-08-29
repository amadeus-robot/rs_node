use rocksdb::{DB, Options, WriteBatch};
use std::env;
use std::fs;
use std::sync::Arc;

pub struct RocksHandle {
    pub db: Arc<DB>,
    pub cf_contractstate: String, // placeholder, in Rust you’d likely hold ColumnFamily handle
}

pub struct Offline {
    rocks: RocksHandle,
}

impl Offline {
    pub fn new(rocks: RocksHandle) -> Self {
        Self { rocks }
    }

    /// Equivalent of Offline.add_balance/2
    pub fn add_balance(&self, amount: Option<&str>, pk: Option<&str>) {
        let pk = pk.unwrap_or_else(|| {
            &env::var("TRAINER_PK").expect("TRAINER_PK env missing")
        });

        let amount = amount.unwrap_or("1000000000000");

        let key = format!("bic:coin:balance:{}:AMA", pk);
        self.put_state(&key, amount.as_bytes());
    }

    /// Equivalent of Offline.deploy/2
    pub fn deploy(&self, wasmpath: &str, pk: Option<&str>) {
        let pk = pk.unwrap_or_else(|| {
            &env::var("TRAINER_PK").expect("TRAINER_PK env missing")
        });

        let wasmbytes = fs::read(wasmpath).expect("failed to read wasm file");

        let key = format!("bic:contract:account:{}:bytecode", pk);
        self.put_state(&key, &wasmbytes);
    }

    /// Equivalent of Offline.call/6
    pub fn call(
        &self,
        sk: &str,
        pk: &str,
        function: &str,
        args: Vec<String>,
        attach_symbol: Option<&str>,
        attach_amount: Option<&str>,
    ) {
        // TODO: Translate your TX/TXPool/Consensus modules here
        // Example placeholder:
        let packed_tx = TX::build(sk, pk, function, args, attach_symbol, attach_amount);
        TXPool::insert(packed_tx);

        let entry = Consensus::produce_entry(Consensus::chain_height() + 1);
        Fabric::insert_entry(&entry, chrono::Utc::now().timestamp_millis());
        Consensus::apply_entry(&entry);
    }

    /// Equivalent of Offline.produce_entry/1
    pub fn produce_entry(clean_txpool: bool) {
        let entry = Consensus::produce_entry(Consensus::chain_height() + 1);
        Fabric::insert_entry(&entry, chrono::Utc::now().timestamp_millis());
        let result = Consensus::apply_entry(&entry);
        if clean_txpool {
            TXPool::purge_stale();
        }
        result
    }

    /// Equivalent of Offline.state/1
    pub fn state(pk: Option<&str>) {
        // TODO: implement account state query
    }

    fn put_state(&self, key: &str, value: &[u8]) {
        self.rocks
            .db
            .put(key.as_bytes(), value)
            .expect("failed to write to RocksDB");
    }
}