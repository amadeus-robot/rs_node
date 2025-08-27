use rocksdb::{DB, WriteBatch};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::UNIX_EPOCH};

use crate::*;

#[derive(Debug, Clone)]
pub struct MapEnv {
    pub readonly: bool,
    pub seed: Option<Vec<u8>>,
    pub seedf64: f64,
    pub entry_signer: Vec<u8>,
    pub entry_prev_hash: Vec<u8>,
    pub entry_slot: u64,
    pub entry_prev_slot: i64,
    pub entry_height: u64,
    pub entry_epoch: u64,
    pub entry_vr: Vec<u8>,
    pub entry_vr_b3: Vec<u8>,
    pub entry_dr: Vec<u8>,
    pub tx_index: usize,
    pub tx_signer: Option<String>,
    pub tx_nonce: Option<u64>,
    pub tx_hash: Option<Vec<u8>>,
    pub account_origin: Option<String>,
    pub account_caller: Option<String>,
    pub account_current: Option<String>,
    pub attached_symbol: String,
    pub attached_amount: i64,
    pub call_counter: u64,
    pub call_exec_points: u64,
    pub call_exec_points_remaining: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

    pub fn make_mapenv(next_entry: &Entry) -> MapEnv {
        let header = &next_entry.header_unpacked;

        MapEnv {
            readonly: false,
            seed: None,
            seedf64: 1.0,
            entry_signer: header.signer.clone(),
            entry_prev_hash: header.prev_hash.clone(),
            entry_slot: header.slot,
            entry_prev_slot: header.prev_slot,
            entry_height: header.height,
            entry_epoch: header.height / 100_000,
            entry_vr: header.vr.clone(),
            entry_vr_b3: blake3::hash(&header.vr).as_bytes().to_vec(),
            entry_dr: header.dr.clone(),
            tx_index: 0,
            tx_signer: None,
            tx_nonce: None,
            tx_hash: None,
            account_origin: None,
            account_caller: None,
            account_current: None,
            attached_symbol: "".to_string(),
            attached_amount: 0,
            call_counter: 0,
            call_exec_points: 10_000_000,
            call_exec_points_remaining: 10_000_000,
        }
    }

    // pub fn validate_vs_chain(&mut self) -> Result<(), String> {
    //     let to_sign = [self.entry_hash.clone(), self.mutations_hash.clone()].concat();

    //     let entry = Fabric::entry_by_hash(&self.entry_hash).ok_or("invalid_entry")?;

    //     if entry.header_unpacked.height > Self::chain_height() {
    //         return Err("too_far_in_future".into());
    //     }

    //     let trainers = Self::trainers_for_height(entry.header_unpacked.height);
    //     let score = BLS12AggSig::score(&trainers, &self.mask);

    //     let trainers_signed = BLS12AggSig::unmask_trainers(&trainers, &self.mask);
    //     let aggpk = BlsEx::aggregate_public_keys(&trainers_signed);

    //     if !BlsEx::verify(&aggpk, &self.aggsig, &to_sign, BLS12AggSig::dst_att()) {
    //         return Err("invalid_signature".into());
    //     }

    //     self.score = Some(score);
    //     Ok(())
    // }

    // pub fn chain_height() -> u64 {
    //      Self::chain_tip_entry().header_unpacked.height
    // }

    pub fn trainers_for_height(height: u64) -> Vec<Vec<u8>> {
        // TODO: implement DB access for trainers
        vec![]
    }

    pub fn trainer_for_slot(height: u64, slot: u64) -> Vec<u8> {
        let trainers = Self::trainers_for_height(height);
        let index = (slot % trainers.len() as u64) as usize;
        trainers[index].clone()
    }

    pub fn chain_height() -> u64 {
        let entry = Self::chain_tip_entry();

        entry.header_unpacked.height
    }

    pub fn chain_epoch() -> u64 {
        Self::chain_tip_entry().header_unpacked.height / 100_000
    }

    pub fn chain_tip_entry() -> Entry {
        //  ADD GET FABRIC FROM DB

        Entry::unpack(None).unwrap()
    }

    // pub fn apply_entry_1(
    //     next_entry: &Entry,
    //     cf: &ColumnFamilies,
    //     db: &DB,
    // )  {
    // ) -> Result<ApplyResult, &'static str> {
    // // Start a RocksDB transaction
    // let mut rtx = WriteBatch::default();

    // // 1. Create mapenv from entry
    // let mut mapenv = Self::make_mapenv(next_entry);

    // // 2. Unpack transactions
    // let txus: Vec<Txu> = next_entry.txs.iter().map(|tx| TX::unpack(tx)).collect();

    // // 3. Pre-parallel call
    // let (mut m_pre, mut m_rev_pre) = Base::call_txs_pre_parallel(mapenv, txus);

    // // 4. Iterate transactions sequentially
    // let mut m = m_pre.clone();
    // let mut m_rev = m_rev_pre.clone();
    // let mut logs = Vec::new();

    // for (tx_idx, txu) in txus.iter().enumerate() {
    //     let mut local_env = mapenv.clone();

    //     local_env.tx_index = tx_idx.into();
    //     local_env.tx_signer = Some(txu.tx.unwrap().signer);
    //     local_env.tx_hash = txu.hash.clone().into();
    //     local_env.account_origin = txu.tx.unwrap().signer.clone().into();
    //     local_env.account_caller = txu.tx.unwrap().signer.clone().into();

    // let (m3, m_rev3, m3_gas, m3_gas_rev, result) =
    //     Base::call_tx_actions(local_env, txu.clone());

    // if result.error == "ok" {
    //     m.extend(m3.into_iter().chain(m3_gas.into_iter()));
    //     m_rev.extend(m_rev3.into_iter().chain(m3_gas_rev.into_iter()));
    //     logs.push(result);
    // } else {
    //     ConsensusKV::revert(&m_rev3);
    //     m.extend(m3_gas);
    //     m_rev.extend(m3_gas_rev);
    //     logs.push(result);
    // }
    // }

    // // 5. Call exit
    // let (m_exit, m_exit_rev) = BIC::Base::call_exit(&mapenv);
    // m.extend(m_exit);
    // m_rev.extend(m_exit_rev);

    // // 6. Compute mutations hash
    // let mutations_hash = ConsensusKV::hash_mutations(&logs, &m);

    // // 7. Attestation
    // let attestation = Attestation::sign(&next_entry.hash, &mutations_hash);
    // let attestation_packed = Attestation::pack(&attestation);
    // db.put_cf(
    //     &cf.my_attestation_for_entry,
    //     &next_entry.hash,
    //     &attestation_packed,
    // )
    // .unwrap();

    // // 8. Trainer check
    // let pk = AMACONFIG.trainer_pk.clone();
    // let trainers = trainers_for_height(next_entry.height(), &cf, &db);
    // let is_trainer = trainers.contains(&pk);

    // // 9. Seen time
    // let seen_time = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis() as u64;
    // db.put_cf(
    //     &cf.my_seen_time_for_entry,
    //     &next_entry.hash,
    //     &bincode::serialize(&seen_time).unwrap(),
    // )
    // .unwrap();

    // // 10. Update temporal tip and height
    // db.put_cf(&cf.sysconf, b"temporal_tip", &next_entry.hash)
    //     .unwrap();
    // db.put_cf(
    //     &cf.sysconf,
    //     b"temporal_height",
    //     &bincode::serialize(&next_entry.header_unpacked.height).unwrap(),
    // )
    // .unwrap();

    // // 11. Reverse mutations
    // db.put_cf(
    //     &cf.muts_rev,
    //     &next_entry.hash,
    //     &bincode::serialize(&m_rev).unwrap(),
    // )
    // .unwrap();

    // // 12. Store tx results
    // let entry_packed = db.get_cf(&cf.default, &next_entry.hash).unwrap().unwrap();
    // for (tx_packed, result) in next_entry.txs.iter().zip(logs.iter()) {
    //     let txu = TX::unpack(tx_packed);
    //     if let Some((index_start, _size)) = find_binary(entry_packed.as_slice(), tx_packed) {
    //         let value = bincode::serialize(&TxResultBinary {
    //             entry_hash: next_entry.hash.clone(),
    //             result: result.clone(),
    //             index_start,
    //         })
    //         .unwrap();
    //         db.put_cf(&cf.tx, &txu.hash, &value).unwrap();

    //         let nonce_padded = format!("{:0>20}", txu.tx.nonce);
    //         db.put_cf(
    //             &cf.tx_account_nonce,
    //             format!("{}:{}", txu.tx.signer, nonce_padded),
    //             &txu.hash,
    //         )
    //         .unwrap();

    //         for receiver in TX::known_receivers(&txu) {
    //             db.put_cf(
    //                 &cf.tx_receiver_nonce,
    //                 format!("{}:{}", receiver, nonce_padded),
    //                 &txu.hash,
    //             )
    //             .unwrap();
    //         }
    //     }
    // }

    // // 13. Archival node
    // if AMACONFIG.archival_node {
    //     db.put_cf(&cf.muts, &next_entry.hash, &bincode::serialize(&m).unwrap())
    //         .unwrap();
    // }

    // // 14. Commit transaction
    // // RocksDB WriteBatch applied manually
    // db.write(rtx).unwrap();

    // // 15. Optional attestation dispatch
    // let ap = if is_trainer {
    //     FabricCoordinatorGen::send_add_attestation(&attestation);
    //     Some(attestation_packed.clone())
    // } else {
    //     None
    // };

    // Ok(ApplyResult {
    //     error: "ok".to_string(),
    //     attestation_packed: ap,
    //     mutations_hash,
    //     logs,
    //     muts: m,
    // })

    // }
}
