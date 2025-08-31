use std::cell::RefCell;

use blake3;
use rocksdb::{MultiThreaded, Transaction, TransactionDB, WriteOptions};
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Clone, Serialize, Deserialize)]
pub enum Mutation {
    Put {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Delete {
        key: Vec<u8>,
    },
    SetBit {
        key: Vec<u8>,
        bit_idx: usize,
        bloomsize: usize,
    },
    ClearBit {
        key: Vec<u8>,
        bit_idx: usize,
    },
}

pub struct ConsensusKV;

impl ConsensusKV {
    pub fn kv_put(key: Vec<u8>, value: Vec<u8>) -> Result<(), rocksdb::Error> {
        // let fabric = FABRIC_DB.read().unwrap();
        // let fabric = fabric.as_ref().expect("Fabric not initialized");

        // let old_value = fabric.db.get(&key)?.unwrap_or_default();
        // let exists = !old_value.is_empty();

        // MUTATIONS.with(|mutations| {
        //     mutations.borrow_mut().push(Mutation::Put {
        //         key: key.clone(),
        //         value: value.clone(),
        //     });
        // });
        // if exists {
        //     MUTATIONS_REVERSE.with(|mutations_reverse| {
        //         mutations_reverse.borrow_mut().push(Mutation::Put {
        //             key: key.clone(),
        //             value: old_value.clone(),
        //         })
        //     });
        // } else {
        //     MUTATIONS_REVERSE.with(|mutations_reverse| {
        //         mutations_reverse
        //             .borrow_mut()
        //             .push(Mutation::Delete { key: key.clone() })
        //     });
        // }

        // fabric.db.put(&key, value)?;
        Ok(())
    }

    pub fn kv_increment(key: Vec<u8>, value: i64) -> Result<(), rocksdb::Error> {
        // pub fn kv_increment(key: Vec<u8>, value: i64) -> Result<i64, rocksdb::Error> {
        // let fabric = FABRIC_DB.read().unwrap();
        // let fabric = fabric.as_ref().expect("Fabric not initialized");

        // let old_value = fabric
        //     .db
        //     .get(&key)?
        //     .unwrap_or_else(|| 0i64.to_be_bytes().to_vec());
        // let exists = !old_value.is_empty();

        // let old_int = i64::from_be_bytes(old_value.clone().try_into().unwrap());
        // let new_value = old_int + value;
        // let new_bytes = new_value.to_be_bytes().to_vec();

        // MUTATIONS.with(|mutations| {
        //     mutations.borrow_mut().push(Mutation::Put {
        //         key: key.clone(),
        //         value: new_bytes.clone(),
        //     });
        // });
        // if exists {
        //     MUTATIONS_REVERSE.with(|mutations_reverse| {
        //         mutations_reverse.borrow_mut().push(Mutation::Put {
        //             key: key.clone(),
        //             value: old_value.clone(),
        //         });
        //     });
        // } else {
        //     MUTATIONS_REVERSE.with(|mutations_reverse| {
        //         mutations_reverse
        //             .borrow_mut()
        //             .push(Mutation::Delete { key: key.clone() });
        //     });
        // }

        // fabric.db.put(&key, new_bytes.clone())?;
        // Ok(new_value)
        Ok(())
    }

    pub fn kv_delete(
        tx: &mut Transaction<TransactionDB<MultiThreaded>>,
        mutations: &mut Vec<Mutation>,
        mutations_reverse: &mut Vec<Mutation>,
        key: Vec<u8>,
    ) -> Result<(), rocksdb::Error> {
        if let Some(value) = tx.get(&key)? {
            mutations.push(Mutation::Delete { key: key.clone() });
            mutations_reverse.push(Mutation::Put {
                key: key.clone(),
                value,
            });
        }
        tx.delete(&key)?;
        Ok(())
    }

    pub fn kv_get(key: &[u8]) -> Option<Vec<u8>> {
        let fabric = FABRIC_DB.read().unwrap();
        let fabric = fabric.as_ref().expect("Fabric not initialized");

        fabric.db.get(key).unwrap()
    }

    // pub fn kv_set_bit(
    //     tx: &mut Transaction<TransactionDB<MultiThreaded>>,
    //     mutations: &mut Vec<Mutation>,
    //     mutations_reverse: &mut Vec<Mutation>,
    //     key: Vec<u8>,
    //     bit_idx: usize,
    //     bloomsize: usize,
    // ) -> Result<bool, rocksdb::Error> {
    //     let old_value: Vec<u8> = tx.get(&key)?.unwrap_or_else(|| vec![0u8; bloomsize]);
    //     let mut bits = BitVec::<Msb0, u8>::from_vec(old_value.clone());

    //     if bits.get(bit_idx).copied().unwrap_or(false) {
    //         return Ok(false);
    //     }

    //     bits.set(bit_idx, true);
    //     let new_bytes = bits.into_vec();

    //     mutations.push(Mutation::SetBit {
    //         key: key.clone(),
    //         bit_idx,
    //         bloomsize,
    //     });
    //     mutations_reverse.push(Mutation::ClearBit {
    //         key: key.clone(),
    //         bit_idx,
    //     });

    //     tx.put(&key, new_bytes)?;
    //     Ok(true)
    // }

    pub fn hash_mutations(mutations: &[Mutation]) -> Vec<u8> {
        let bin = bincode::serialize(mutations).unwrap();
        blake3::hash(&bin).as_bytes().to_vec()
    }

    // pub fn revert(
    //     tx: &mut Transaction<TransactionDB<MultiThreaded>>,
    //     mutations_reverse: &[Mutation],
    // ) -> Result<(), rocksdb::Error> {
    //     for mut_item in mutations_reverse.iter().rev() {
    //         match mut_item {
    //             Mutation::Put { key, value } => tx.put(key, value)?,
    //             Mutation::Delete { key } => tx.delete(key)?,
    //             Mutation::ClearBit { key, bit_idx } => {
    //                 if let Some(old_value) = tx.get(key)? {
    //                     let mut bits = BitVec::<Msb0, u8>::from_vec(old_value);
    //                     bits.set(*bit_idx, false);
    //                     tx.put(key, bits.into_vec())?;
    //                 }
    //             }
    //             _ => {}
    //         }
    //     }
    //     Ok(())
    // }

    // pub fn merge_nested(
    //     left: HashMap<String, serde_json::Value>,
    //     right: HashMap<String, serde_json::Value>,
    // ) -> HashMap<String, serde_json::Value> {
    //     let mut merged = left.clone();
    //     for (k, v) in right {
    //         merged
    //             .entry(k)
    //             .and_modify(|old| {
    //                 if old.is_object() && v.is_object() {
    //                     *old = serde_json::Value::Object(Self::merge_nested(
    //                         old.as_object().unwrap().clone(),
    //                         v.as_object().unwrap().clone(),
    //                     ));
    //                 } else {
    //                     *old = v.clone();
    //                 }
    //             })
    //             .or_insert(v);
    //     }
    //     merged
    // }
}
