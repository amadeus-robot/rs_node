// use rocksdb::{DB, Options, IteratorMode, WriteBatch, ColumnFamily, DBWithThreadMode, SingleThreaded, Direction};
// use std::path::Path;

// pub struct RocksDB {
//     pub db: DBWithThreadMode<SingleThreaded>,
//     pub cfs: Option<Vec<String>>, // optional column families
// }

// impl RocksDB {
//     pub fn get(&self, key: &[u8], term: bool, to_integer: bool, cf: Option<&str>) -> Option<Vec<u8>> {
//         let val = match cf {
//             Some(cf_name) => {
//                 let cf_handle = self.db.cf_handle(cf_name)?;
//                 self.db.get_cf(cf_handle, key).ok()?
//             },
//             None => self.db.get(key).ok()?,
//         };
//         val.map(|v| {
//             if term {
//                 bincode::deserialize::<Vec<u8>>(&v).unwrap_or_default()
//             } else if to_integer {
//                 let mut arr = [0u8; 8];
//                 arr.copy_from_slice(&v[..8.min(v.len())]);
//                 (u64::from_be_bytes(arr)).to_le_bytes().to_vec()
//             } else {
//                 v
//             }
//         })
//     }

//     pub fn put(&self, key: &[u8], value: &[u8], term: bool, to_integer: bool, cf: Option<&str>) {
//         let val = if term {
//             bincode::serialize(value).unwrap()
//         } else if to_integer {
//             let num = u64::from_le_bytes(value.try_into().unwrap_or([0u8;8]));
//             num.to_be_bytes().to_vec()
//         } else {
//             value.to_vec()
//         };

//         match cf {
//             Some(cf_name) => {
//                 if let Some(cf_handle) = self.db.cf_handle(cf_name) {
//                     self.db.put_cf(cf_handle, key, val).unwrap();
//                 }
//             },
//             None => { self.db.put(key, val).unwrap(); }
//         }
//     }

//     pub fn delete(&self, key: &[u8], cf: Option<&str>) {
//         match cf {
//             Some(cf_name) => {
//                 if let Some(cf_handle) = self.db.cf_handle(cf_name) {
//                     self.db.delete_cf(cf_handle, key).unwrap();
//                 }
//             },
//             None => { self.db.delete(key).unwrap(); }
//         }
//     }

//     // pub fn get_prefix(&self, prefix: &[u8], term: bool, to_integer: bool, cf: Option<&str>) -> Vec<(Vec<u8>, Vec<u8>)> {
//     //     let mode = IteratorMode::From(prefix, Direction::Forward);
//     //     let iter: Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)>> = match cf {
//     //         Some(cf_name) => {
//     //             let cf_handle = self.db.cf_handle(cf_name).unwrap();
//     //             Box::new(self.db.iterator_cf(cf_handle, mode))
//     //         },
//     //         None => Box::new(self.db.iterator(mode))
//     //     };

//     //     iter.take_while(|(k, _v)| k.starts_with(prefix))
//     //         .map(|(k, v)| {
//     //             let value = if term {
//     //                 bincode::deserialize::<Vec<u8>>(&v).unwrap_or_default()
//     //             } else if to_integer {
//     //                 let mut arr = [0u8; 8];
//     //                 arr.copy_from_slice(&v[..8.min(v.len())]);
//     //                 (u64::from_be_bytes(arr)).to_le_bytes().to_vec()
//     //             } else { v.to_vec() };
//     //             (k.to_vec(), value)
//     //         })
//     //         .collect()
//     // }

//     // pub fn dump(&self, cf: Option<&str>) {
//     //     let iter: Box<dyn Iterator<Item = (Box<[u8]>, Box<[u8]>)>> = match cf {
//     //         Some(cf_name) => {
//     //             let cf_handle = self.db.cf_handle(cf_name).unwrap();
//     //             Box::new(self.db.iterator_cf(cf_handle, IteratorMode::Start))
//     //         },
//     //         None => Box::new(self.db.iterator(IteratorMode::Start))
//     //     };

//     //     for (k, v) in iter {
//     //         println!("{:?} => {:?}", k, v);
//     //     }
//     // }

//     pub fn flush_all(&self) {
//         self.db.flush().unwrap();
//         if let Some(cfs) = &self.cfs {
//             for cf_name in cfs {
//                 if let Some(cf_handle) = self.db.cf_handle(cf_name) {
//                     self.db.flush_cf(cf_handle).unwrap();
//                 }
//             }
//         }
//     }

//     pub fn compact_all(&self) {
//         self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
//         if let Some(cfs) = &self.cfs {
//             for cf_name in cfs {
//                 if let Some(cf_handle) = self.db.cf_handle(cf_name) {
//                     self.db.compact_range_cf(cf_handle, None::<&[u8]>, None::<&[u8]>);
//                 }
//             }
//         }
//     }

//     // pub fn get_lru(&self) -> (u64, u64) {
//     //     let used = self.db.get_property_int("rocksdb.block-cache-usage").unwrap_or(0);
//     //     let cap = self.db.get_property_int("rocksdb.block-cache-capacity").unwrap_or(0);
//     //     (used, cap)
//     // }

//     pub fn checkpoint(&self, path: &str) {
//         let cp = rocksdb::checkpoint::Checkpoint::new(&self.db).unwrap();
//         cp.create_checkpoint(Path::new(path)).unwrap();
//     }

//     // Snapshot and restore can be implemented via filesystem copy or checkpoint
//     pub fn snapshot(&self, path: &str) {
//         self.checkpoint(path);
//     }
// }
