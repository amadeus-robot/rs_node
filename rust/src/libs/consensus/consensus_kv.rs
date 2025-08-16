use rocksdb::{Transaction, TransactionDB, WriteOptions};
use bitvec::prelude::*;
use serde::{Serialize, Deserialize};
use blake3;
use std::collections::HashMap;

use crate::*;

#[derive(Clone)]
pub enum Mutation {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
    SetBit { key: Vec<u8>, bit_idx: usize, bloomsize: usize },
    ClearBit { key: Vec<u8>, bit_idx: usize },
}

pub struct ConsensusKV<'a> {
    pub db: &'a TransactionDB,
    pub tx: Transaction<'a, TransactionDB>,
    pub mutations: Vec<Mutation>,
    pub mutations_reverse: Vec<Mutation>,
}

impl<'a> ConsensusKV<'a> {
    pub fn kv_put(&mut self, key: Vec<u8>, value: Vec<u8>, term: bool, to_integer: bool) -> Result<(), rocksdb::Error> {
        let old_value = self.tx.get(&key)?.unwrap_or_default();
        let exists = !old_value.is_empty();

        let mut value = value.clone();
        if term {
            value = bincode::serialize(&value).unwrap();
        }
        if to_integer {
            let int_val: i64 = String::from_utf8(value.clone()).unwrap().parse().unwrap();
            value = int_val.to_be_bytes().to_vec();
        }

        self.mutations.push(Mutation::Put { key: key.clone(), value: value.clone() });
        if exists {
            self.mutations_reverse.push(Mutation::Put { key: key.clone(), value: old_value.clone() });
        } else {
            self.mutations_reverse.push(Mutation::Delete { key: key.clone() });
        }

        self.tx.put(&key, value)?;
        Ok(())
    }

    pub fn kv_increment(&mut self, key: Vec<u8>, value: i64) -> Result<i64, rocksdb::Error> {
        let old_value = self.tx.get(&key)?.unwrap_or_else(|| 0i64.to_be_bytes().to_vec());
        let exists = !old_value.is_empty();

        let old_int = i64::from_be_bytes(old_value.try_into().unwrap());
        let new_value = old_int + value;
        let new_bytes = new_value.to_be_bytes().to_vec();

        self.mutations.push(Mutation::Put { key: key.clone(), value: new_bytes.clone() });
        if exists {
            self.mutations_reverse.push(Mutation::Put { key: key.clone(), value: old_value.clone() });
        } else {
            self.mutations_reverse.push(Mutation::Delete { key: key.clone() });
        }

        self.tx.put(&key, new_bytes.clone())?;
        Ok(new_value)
    }

    pub fn kv_delete(&mut self, key: Vec<u8>) -> Result<(), rocksdb::Error> {
        if let Some(value) = self.tx.get(&key)? {
            self.mutations.push(Mutation::Delete { key: key.clone() });
            self.mutations_reverse.push(Mutation::Put { key: key.clone(), value });
        }
        self.tx.delete(&key)?;
        Ok(())
    }

    pub fn kv_get(&self, key: &[u8], term: bool, to_integer: bool) -> Option<Vec<u8>> {
        if let Ok(Some(value)) = self.tx.get(key) {
            if term {
                return Some(bincode::deserialize(&value).unwrap());
            } else if to_integer {
                let int_val = i64::from_be_bytes(value.try_into().unwrap());
                return Some(int_val.to_be_bytes().to_vec());
            } else {
                return Some(value);
            }
        }
        None
    }

    pub fn kv_set_bit(&mut self, key: Vec<u8>, bit_idx: usize, bloomsize: usize) -> Result<bool, rocksdb::Error> {
        let old_value = self.tx.get(&key)?.unwrap_or_else(|| vec![0u8; bloomsize]);
        let mut bits = BitVec::<Msb0, u8>::from_vec(old_value.clone());

        if bits.get(bit_idx).copied().unwrap_or(false) {
            return Ok(false);
        }

        bits.set(bit_idx, true);
        let new_bytes = bits.into_vec();

        self.mutations.push(Mutation::SetBit { key: key.clone(), bit_idx, bloomsize });
        self.mutations_reverse.push(Mutation::ClearBit { key: key.clone(), bit_idx });

        self.tx.put(&key, new_bytes)?;
        Ok(true)
    }

    pub fn hash_mutations(mutations: &[Mutation]) -> Vec<u8> {
        let bin = bincode::serialize(mutations).unwrap();
        blake3::hash(&bin).as_bytes().to_vec()
    }

    pub fn revert(&mut self) -> Result<(), rocksdb::Error> {
        for mut_item in self.mutations_reverse.iter().rev() {
            match mut_item {
                Mutation::Put { key, value } => self.tx.put(key, value)?,
                Mutation::Delete { key } => self.tx.delete(key)?,
                Mutation::ClearBit { key, bit_idx } => {
                    if let Some(old_value) = self.tx.get(key)? {
                        let mut bits = BitVec::<Msb0, u8>::from_vec(old_value);
                        bits.set(*bit_idx, false);
                        self.tx.put(key, bits.into_vec())?;
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub fn merge_nested(left: HashMap<String, serde_json::Value>, right: HashMap<String, serde_json::Value>) -> HashMap<String, serde_json::Value> {
        let mut merged = left.clone();
        for (k, v) in right {
            merged.entry(k).and_modify(|old| {
                if old.is_object() && v.is_object() {
                    *old = serde_json::Value::Object(Self::merge_nested(
                        old.as_object().unwrap().clone(),
                        v.as_object().unwrap().clone(),
                    ));
                } else {
                    *old = v.clone();
                }
            }).or_insert(v);
        }
        merged
    }
}
