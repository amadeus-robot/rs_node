use rocksdb::{DB, IteratorMode, ColumnFamily, Options};
use std::fs::OpenOptions;
use std::io::Write;

/// Helper functions (similar to Util.hexdump in Elixir)
fn hexdump(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join("")
}

/// KV helpers (similar to ConsensusKV wrapper in Elixir)
fn kv_get_prefix(db: &DB, cf: &ColumnFamily, prefix: &str) -> Vec<(String, Vec<u8>)> {
    let mut result = Vec::new();
    let prefix_bytes = prefix.as_bytes();
    let iter = db.prefix_iterator_cf(cf, prefix_bytes);

    for item in iter {
        if let Ok((k, v)) = item {
            if let Ok(k_str) = String::from_utf8(k.to_vec()) {
                result.push((k_str, v.to_vec()));
            }
        }
    }
    result
}

fn kv_put(db: &DB, cf: &ColumnFamily, key: &str, value: &[u8]) {
    db.put_cf(cf, key.as_bytes(), value).unwrap();
}

fn kv_delete(db: &DB, cf: &ColumnFamily, key: &str) {
    db.delete_cf(cf, key.as_bytes()).unwrap();
}

pub struct BICMigrate;

impl BICMigrate {
    /// Equivalent to Elixir's `test/0`
    pub fn test(db: &DB, cf: &ColumnFamily) {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("/tmp/dump")
            .unwrap();

        let iter = db.iterator_cf(cf, IteratorMode::Start).unwrap();
        for (k, v) in iter {
            let key_hex = hexdump(&k);
            let _ = writeln!(file, "{}", key_hex);
            let _ = file.write_all(&v);
        }
    }

    /// Equivalent to Elixir's `migrate/1`
    pub fn migrate(db: &DB, cf: &ColumnFamily, epoch: i64) {
        if epoch == 103 {
            let kvs = kv_get_prefix(db, cf, "");

            for (k, v) in kvs {
                if k.starts_with("bic:base:nonce:") {
                    kv_delete(db, cf, &k);
                    kv_put(db, cf, &k, v.to_string().as_bytes());
                } else if k.starts_with("bic:coin:balance:") {
                    kv_delete(db, cf, &k);
                    let key = format!("{}:AMA", k);
                    kv_put(db, cf, &key, v.to_string().as_bytes());
                } else if k.starts_with("bic:epoch:emission_address:")
                    || k.starts_with("bic:epoch:pop:")
                    || k.starts_with("bic:epoch:segment_vr")
                    || k.starts_with("bic:epoch:solutions:")
                {
                    kv_delete(db, cf, &k);
                    kv_put(db, cf, &k, &v);
                } else if k.starts_with("bic:epoch:trainers:removed:")
                    || k.starts_with("bic:epoch:trainers:height:")
                    || k.starts_with("bic:epoch:trainers:")
                {
                    // skip (do nothing)
                } else {
                    eprintln!("Unexpected key: {}", hexdump(k.as_bytes()));
                    eprintln!("migration failed");
                    panic!("migration_failed");
                }
            }
        } else {
            // other epoch => no-op
        }
    }
}
