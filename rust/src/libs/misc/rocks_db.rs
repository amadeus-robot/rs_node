use rocksdb::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct RocksDB {
    db: Arc<TransactionDB<MultiThreaded>>,
    cfs: HashMap<String, Arc<ColumnFamily>>, // see note
}

impl RocksDB {
    pub fn open(path: &str, cfs: &[&str]) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let txn_opts = TransactionDBOptions::default();

        let db: Arc<TransactionDB> = Arc::new(TransactionDB::open_cf(&opts, &txn_opts, path, cfs).unwrap());

        let mut cf_map = HashMap::new();
        for &cf_name in cfs {
            let cf = db.cf_handle(cf_name).unwrap();
            cf_map.insert(cf_name.to_string(), cf.clone()); // clone Arc
        }

        Self { db, cfs: cf_map }
    }

    pub fn get(&self, key: &[u8], cf: Option<&str>, to_integer: bool) -> Option<Vec<u8>> {
        let value: Option<DBVector> = match cf {
            Some(cf_name) => self.db.get_cf(self.cfs.get(cf_name).unwrap(), key).unwrap(),
            None => self.db.get(key).unwrap(),
        };
        value.map(|v| {
            if to_integer {
                let int_val = i64::from_be_bytes(v[..8].try_into().unwrap());
                int_val.to_be_bytes().to_vec()
            } else {
                v.to_vec()
            }
        })
    }

    pub fn put(&self, key: &[u8], value: &[u8], cf: Option<&str>) {
        match cf {
            Some(cf_name) => self
                .db
                .put_cf(self.cfs.get(cf_name).unwrap(), key, value)
                .unwrap(),
            None => self.db.put(key, value).unwrap(),
        }
    }

    pub fn delete(&self, key: &[u8], cf: Option<&str>) {
        match cf {
            Some(cf_name) => self
                .db
                .delete_cf(self.cfs.get(cf_name).unwrap(), key)
                .unwrap(),
            None => self.db.delete(key).unwrap(),
        }
    }

    pub fn get_prefix(
        &self,
        prefix: &[u8],
        cf: Option<&str>,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, rocksdb::Error> {
        let iter = match cf {
            Some(cf_name) => self
                .db
                .iterator_cf(self.cfs.get(cf_name).unwrap(), IteratorMode::Start),
            None => self.db.iterator(IteratorMode::Start),
        };

        let mut results = Vec::new();

        for item in iter {
            let (k, v) = item?; // propagate RocksDB error
            if k.starts_with(prefix) {
                results.push((k.to_vec(), v.to_vec()));
            }
        }

        Ok(results)
    }

    pub fn get_next(
        &self,
        prefix: &[u8],
        key: &[u8],
        cf: Option<&str>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, rocksdb::Error> {
        let iter = match cf {
            Some(cf_name) => self.db.iterator_cf(
                self.cfs.get(cf_name).unwrap(),
                IteratorMode::From(key, Direction::Forward),
            ),
            None => self
                .db
                .iterator(IteratorMode::From(key, Direction::Forward)),
        };

        for item in iter {
            let (k, v) = item?; // propagate RocksDB error
            if k.starts_with(prefix) {
                if &k[..] != key {
                    return Ok(Some((k.to_vec(), v.to_vec())));
                }
            }
        }

        Ok(None)
    }

    pub fn get_prev(
        &self,
        prefix: &[u8],
        key: &[u8],
        cf: Option<&str>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>, rocksdb::Error> {
        let iter = match cf {
            Some(cf_name) => self.db.iterator_cf(
                self.cfs.get(cf_name).unwrap(),
                IteratorMode::From(key, Direction::Reverse),
            ),
            None => self
                .db
                .iterator(IteratorMode::From(key, Direction::Reverse)),
        };

        for item in iter {
            let (k, v) = item?; // propagate RocksDB error
            if k.starts_with(prefix) {
                if &k[..] != key {
                    // skip the starting key itself
                    return Ok(Some((k.to_vec(), v.to_vec())));
                }
            }
        }

        Ok(None)
    }

    pub fn flush_all(&self) {
        for cf in self.cfs.values() {
            self.db.flush_cf(cf).unwrap();
        }
    }

    pub fn compact_all(&self) {
        for cf in self.cfs.values() {
            self.db.compact_range_cf(cf, None::<&[u8]>, None::<&[u8]>);
        }
    }

    pub fn get_lru(&self) -> Result<(usize, usize), &'static str> {
        Ok((
            self.db
                .property_int_value("rocksdb.block-cache-usage")
                .ok_or("Failed to read block-cache-usage")? as usize,
            self.db
                .property_int_value("rocksdb.block-cache-capacity")
                .ok_or("Failed to read block-cache-capacity")? as usize,
        ))
    }

    pub fn checkpoint(&self, path: &str) {
        let checkpoint = rocksdb::checkpoint::Checkpoint::new(&self.db).unwrap();
        checkpoint.create_checkpoint(path).unwrap();
    }
}
