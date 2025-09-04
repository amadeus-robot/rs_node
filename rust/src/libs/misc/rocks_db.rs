use rocksdb::{ColumnFamily, DB, Direction, IteratorMode, Options, WriteBatch};
use std::collections::HashMap;
use std::path::Path;

pub struct RocksDB {
    db: DB,
    cfs: HashMap<String, ColumnFamily>,
}

impl RocksDB {
    pub fn open(path: &str, cfs: &[&str]) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = DB::open_cf(&opts, path, cfs).unwrap();
        let mut cf_map = HashMap::new();
        for &cf_name in cfs {
            let cf = db.cf_handle(cf_name).unwrap();
            cf_map.insert(cf_name.to_string(), cf);
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

    pub fn get_prefix(&self, prefix: &[u8], cf: Option<&str>) -> Vec<(Vec<u8>, Vec<u8>)> {
        let iter = match cf {
            Some(cf_name) => self
                .db
                .iterator_cf(self.cfs.get(cf_name).unwrap(), IteratorMode::Start),
            None => self.db.iterator(IteratorMode::Start),
        };

        iter.filter_map(|(k, v)| {
            if k.starts_with(prefix) {
                Some((k.to_vec(), v.to_vec()))
            } else {
                None
            }
        })
        .collect()
    }

    pub fn get_next(
        &self,
        prefix: &[u8],
        key: &[u8],
        cf: Option<&str>,
    ) -> Option<(Vec<u8>, Vec<u8>)> {
        let iter = match cf {
            Some(cf_name) => self.db.iterator_cf(
                self.cfs.get(cf_name).unwrap(),
                IteratorMode::From(key, Direction::Forward),
            ),
            None => self
                .db
                .iterator(IteratorMode::From(key, Direction::Forward)),
        };

        iter.filter(|(k, _)| k.starts_with(prefix))
            .skip(1) // equivalent of offset=1 in Elixir
            .next()
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
    }

    pub fn get_prev(
        &self,
        prefix: &[u8],
        key: &[u8],
        cf: Option<&str>,
    ) -> Option<(Vec<u8>, Vec<u8>)> {
        let iter = match cf {
            Some(cf_name) => self.db.iterator_cf(
                self.cfs.get(cf_name).unwrap(),
                IteratorMode::From(key, Direction::Reverse),
            ),
            None => self
                .db
                .iterator(IteratorMode::From(key, Direction::Reverse)),
        };

        iter.filter(|(k, _)| k.starts_with(prefix))
            .skip(1)
            .next()
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
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

    pub fn get_lru(&self) -> (usize, usize) {
        let used = self
            .db
            .get_property_int("rocksdb.block-cache-usage")
            .unwrap_or(0);
        let capacity = self
            .db
            .get_property_int("rocksdb.block-cache-capacity")
            .unwrap_or(0);
        (used as usize, capacity as usize)
    }

    pub fn checkpoint(&self, path: &str) {
        let checkpoint = rocksdb::checkpoint::Checkpoint::new(&self.db).unwrap();
        checkpoint.create_checkpoint(path).unwrap();
    }
}
