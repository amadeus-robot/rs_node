use std::collections::HashMap;

use crate::*;

pub struct Consensus {
    pub entry_hash: Vec<u8>,
    pub mutations_hash: Vec<u8>,
    pub mask: Vec<u8>,
    pub aggsig: Vec<u8>,
    pub score: Option<f64>,
}

impl Consensus {
    pub fn unpack(data: &[u8]) -> Self {
        // In Rust, use bincode or serde_cbor instead of :erlang.binary_to_term
        bincode::deserialize::<Consensus>(data).unwrap()
    }

    pub fn pack(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn validate_vs_chain(&mut self) -> Result<(), String> {
        let to_sign = [self.entry_hash.clone(), self.mutations_hash.clone()].concat();

        let entry = Fabric::entry_by_hash(&self.entry_hash).ok_or("invalid_entry")?;

        if entry.header_unpacked.height > Self::chain_height() {
            return Err("too_far_in_future".into());
        }

        let trainers = Self::trainers_for_height(entry.header_unpacked.height);
        let score = BLS12AggSig::score(&trainers, &self.mask);

        let trainers_signed = BLS12AggSig::unmask_trainers(&trainers, &self.mask);
        let aggpk = BlsEx::aggregate_public_keys(&trainers_signed);

        if !BlsEx::verify(&aggpk, &self.aggsig, &to_sign, BLS12AggSig::dst_att()) {
            return Err("invalid_signature".into());
        }

        self.score = Some(score);
        Ok(())
    }

    pub fn chain_height() -> u64 {
        Self::chain_tip_entry().header_unpacked.height
    }

    pub fn trainers_for_height(height: u64) -> Vec<Vec<u8>> {
        // TODO: implement DB access for trainers
        vec![]
    }

    pub fn trainer_for_slot(height: u64, slot: u64) -> Vec<u8> {
        let trainers = Self::trainers_for_height(height);
        let index = (slot % trainers.len() as u64) as usize;
        trainers[index].clone()
    }

    pub fn produce_entry(slot: u64) -> Entry {
        let cur_entry = Self::chain_tip_entry();
        let mut next_entry = Entry::build_next(&cur_entry, slot);

        let txs = TXPool::grab_next_valid(100);
        next_entry.txs = txs;
        Entry::sign(&mut next_entry);
        next_entry
    }

    pub fn chain_tip_entry() -> Entry {
        // TODO: fetch tip entry from RocksDB
        Entry::default()
    }
}
