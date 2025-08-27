use anyhow::Result;
use once_cell::sync::Lazy;
use rocksdb::{
    BoundColumnFamily, ColumnFamilyDescriptor, MultiThreaded, Options, TransactionDB,
    TransactionDBOptions,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::*;

pub static FABRIC_DB: Lazy<RwLock<Option<Arc<Fabric>>>> = Lazy::new(|| RwLock::new(None));

pub struct Fabric {
    pub db: &'static Arc<TransactionDB<MultiThreaded>>,
    pub cf: HashMap<&'static str, Arc<BoundColumnFamily<'static>>>,
    pub path: String,
}

pub struct ColumnFamilies {
    pub default: Arc<BoundColumnFamily<'static>>,
    pub entry_by_height: Arc<BoundColumnFamily<'static>>,
    pub entry_by_slot: Arc<BoundColumnFamily<'static>>,
    pub tx: Arc<BoundColumnFamily<'static>>,
    pub tx_account_nonce: Arc<BoundColumnFamily<'static>>,
    pub tx_receiver_nonce: Arc<BoundColumnFamily<'static>>,
    pub my_seen_time_for_entry: Arc<BoundColumnFamily<'static>>,
    pub my_attestation_for_entry: Arc<BoundColumnFamily<'static>>,
    pub consensus: Arc<BoundColumnFamily<'static>>,
    pub consensus_by_entryhash: Arc<BoundColumnFamily<'static>>,
    pub contractstate: Arc<BoundColumnFamily<'static>>,
    pub muts: Arc<BoundColumnFamily<'static>>,
    pub muts_rev: Arc<BoundColumnFamily<'static>>,
    pub sysconf: Arc<BoundColumnFamily<'static>>,
}

impl Fabric {
    pub fn init() -> Result<()> {
        println!("Initing Fabric...");

        // Path to DB
        let mut path = AMACONFIG.work_folder.clone();
        path.push("db");
        path.push("fabric");

        // RocksDB options
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_target_file_size_base(2 * 1024 * 1024 * 1024); // 2GB
        opts.set_target_file_size_multiplier(2);

        let txn_opts = TransactionDBOptions::default();

        // Column family names
        let cf_names: [&'static str; 14] = [
            "default",
            "entry_by_height",
            "entry_by_slot",
            "tx",
            "tx_account_nonce",
            "tx_receiver_nonce",
            "my_seen_time_for_entry",
            "my_attestation_for_entry",
            "consensus",
            "consensus_by_entryhash",
            "contractstate",
            "muts",
            "muts_rev",
            "sysconf",
        ];

        // CF descriptors
        let cf_descriptors: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, opts.clone()))
            .collect();

        // Open DB and wrap in Arc
        let db = Arc::new(TransactionDB::open_cf_descriptors(
            &opts,
            &txn_opts,
            &path,
            cf_descriptors,
        )?);

        // Leak the Arc to get 'static lifetime
        let db_static: &'static Arc<TransactionDB<MultiThreaded>> = Box::leak(Box::new(db));

        // Collect CF handles
        let mut cf_map: HashMap<&'static str, Arc<BoundColumnFamily<'static>>> = HashMap::new();
        for &name in &cf_names {
            let handle = db_static.cf_handle(name).unwrap();
            cf_map.insert(name, handle.clone());
        }

        // Build Fabric struct
        let fabric = Fabric {
            db: db_static,
            cf: cf_map,
            path: path.to_string_lossy().into_owned(),
        };

        // Store globally
        *FABRIC_DB.write().unwrap() = Some(Arc::new(fabric));

        println!("Fabric initialized at {:?}", path);
        Ok(())
    }

    pub fn close() {
        let mut fabric_guard = FABRIC_DB.write().unwrap();

        if fabric_guard.is_some() {
            // Dropping the DB handle will close RocksDB
            *fabric_guard = None;
            println!("Fabric DB closed.");
        } else {
            println!("Fabric DB was not initialized.");
        }
    }

    pub fn rooted_tip() -> Option<Vec<u8>> {
        let fabric_guard = FABRIC_DB.read().unwrap();
        let fabric = fabric_guard.as_ref()?; // return None if DB not initialized

        // Get the "sysconf" column family
        let cf = fabric.db.cf_handle("sysconf")?;

        // Fetch the value from RocksDB
        match fabric.db.get_cf(&cf, b"rooted_tip") {
            Ok(Some(data)) => Some(data),
            Ok(None) => None, // key not found
            Err(err) => {
                eprintln!("RocksDB get error: {}", err);
                None
            }
        }
    }

    pub fn entry_by_hash(hash: Option<&[u8]>) -> Option<Entry> {
        let h = hash?;
        let fabric_guard = FABRIC_DB.read().unwrap();
        let fabric = fabric_guard.as_ref()?;

        // Use default CF for simplicity
        let cf = fabric.db.cf_handle("default")?;
        match fabric.db.get_cf(&cf, h) {
            Ok(Some(data)) => Some(Entry::unpack(Some(&data))), // pass slice and wrap in Some
            Ok(None) => None,
            Err(err) => {
                eprintln!("RocksDB get error: {}", err);
                None
            }
        }
    }

    pub fn rooted_tip_entry() -> Option<Entry> {
        let hash = Self::rooted_tip(); // Option<Vec<u8>>
        Self::entry_by_hash(hash.as_deref()) // convert to Option<&[u8]>
    }

    pub fn rooted_tip_height() -> Option<u64> {
        Self::rooted_tip_entry().map(|entry| entry.header_unpacked.height)
    }

    pub fn pruned_hash() -> Vec<u8> {
        // Get global Fabric DB
        let fabric = FABRIC_DB.read().unwrap();
        let fabric = fabric.as_ref().expect("Fabric not initialized");

        // Access sysconf CF
        let sysconf_cf = fabric.cf.get("sysconf").expect("sysconf CF not found");

        // Try to get "pruned_hash" key
        match fabric.db.get_cf(sysconf_cf, b"pruned_hash") {
            Ok(Some(value)) => value,                     // Found value
            Ok(None) => EntryGenesis::get().hash.clone(), // Default
            Err(e) => {
                panic!("Failed to read pruned_hash: {:?}", e);
            }
        }
    }
}
