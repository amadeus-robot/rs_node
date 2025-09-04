use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::LazyLock; // For global static tables

pub mod tx;
pub use tx::*;

use crate::Consensus;

// Global tables (similar to ETS)
pub static TX_POOL: Lazy<DashMap<u64, String>> = Lazy::new(|| DashMap::new());
pub static GIFTED_SOL_CACHE: Lazy<DashMap<u64, String>> = Lazy::new(|| DashMap::new());

// Assuming you have structs TX and TXU and their methods

// TXPool functions
pub struct TXPool;

impl TXPool {
    pub fn init() {
        println!("Initing TXPool..");

        Lazy::force(&TX_POOL);
        Lazy::force(&GIFTED_SOL_CACHE);
    }

    pub fn insert(packed_tx: Vec<u8>) {
        for tx_packed in txs_packed {
            let txu = Tx::unpack(tx_packed.as_ref());
            TX_POOL.insert((txu.tx.nonce, txu.hash.clone()), txu);
        }
    }

    // pub fn delete_packed<T: AsRef<[u8]>>(txs_packed: &[T]) {
    //     for tx_packed in txs_packed {
    //         let txu = Tx::unpack(tx_packed.as_ref());
    //         TX_POOL.remove(&(txu.tx.nonce, txu.hash));
    //     }
    // }

    // pub fn purge_stale() {
    //     let cur_epoch = Consensus::chain_epoch();
    //     TX_POOL.retain(|_key, txu| !Self::is_stale(txu, cur_epoch));
    // }

    // pub fn grab_next_valid(amt: usize) -> Vec<Vec<u8>> {
    //     let mut acc = Vec::new();
    //     let mut state: HashMap<(&str, Vec<u8>), u64> = HashMap::new();
    //     let chain_epoch = Consensus::chain_epoch();

    //     for item in TX_POOL.iter() {
    //         let txu = item.value().clone();
    //         let signer = txu.tx.signer.clone();

    //         let chain_nonce_key = ("chain_nonce", signer.clone());
    //         let chain_nonce = state
    //             .entry(chain_nonce_key.clone())
    //             .or_insert_with(|| Consensus::chain_nonce(&signer));

    //         if txu.tx.nonce <= *chain_nonce {
    //             TX_POOL.remove(&(txu.tx.nonce, txu.hash.clone()));
    //             continue;
    //         }
    //         *chain_nonce = txu.tx.nonce;

    //         let balance_key = ("balance", signer.clone());
    //         let mut balance = *state
    //             .entry(balance_key.clone())
    //             .or_insert_with(|| Consensus::chain_balance(&signer));
    //         balance = balance.saturating_sub(BIC::Base::exec_cost(&txu));
    //         balance = balance.saturating_sub(BIC::Coin::to_cents(1));
    //         if balance < 0 {
    //             TX_POOL.remove(&(txu.tx.nonce, txu.hash.clone()));
    //             continue;
    //         }
    //         state.insert(balance_key, balance);

    //         let has_sol = txu
    //             .tx
    //             .actions
    //             .iter()
    //             .find(|a| a.function == "submit_sol" && !a.args.is_empty())
    //             .map(|a| &a.args[0]);

    //         let epoch_sol_valid = if let Some(sol) = has_sol {
    //             let sol_epoch = u32::from_le_bytes(sol[0..4].try_into().unwrap());
    //             sol_epoch as u64 == chain_epoch && sol.len() == BIC::Sol::size()
    //         } else {
    //             true
    //         };

    //         if !epoch_sol_valid {
    //             TX_POOL.remove(&(txu.tx.nonce, txu.hash.clone()));
    //             continue;
    //         }

    //         acc.push(Tx::pack(&txu));
    //         if acc.len() == amt {
    //             break;
    //         }
    //     }
    //     acc
    // }

    // pub fn is_stale(txu: &TXU, cur_epoch: u64) -> bool {
    //     let chain_nonce = Consensus::chain_nonce(&txu.tx.signer);
    //     let nonce_valid = txu.tx.nonce > chain_nonce.unwrap_or(0);

    //     let has_sol = txu
    //         .tx
    //         .actions
    //         .iter()
    //         .find(|a| a.function == "submit_sol" && !a.args.is_empty())
    //         .map(|a| &a.args[0]);

    //     let epoch_sol_valid = if let Some(sol) = has_sol {
    //         let sol_epoch = u32::from_le_bytes(sol[0..4].try_into().unwrap());
    //         sol_epoch as u64 == cur_epoch
    //     } else {
    //         true
    //     };

    //     !epoch_sol_valid || !nonce_valid
    // }

    // pub fn random(amount: usize) -> Option<Vec<Vec<u8>>> {
    //     let txs: Vec<_> = TX_POOL
    //         .iter()
    //         .take(amount)
    //         .map(|e| Tx::pack(e.value()))
    //         .collect();
    //     if txs.is_empty() { None } else { Some(txs) }
    // }

    // pub fn lowest_nonce(pk: &[u8]) -> Option<u64> {
    //     TX_POOL
    //         .iter()
    //         .filter(|e| e.value().tx.signer == pk)
    //         .map(|e| e.value().tx.nonce)
    //         .min()
    // }

    // pub fn highest_nonce(pk: &[u8]) -> (Option<u64>, usize) {
    //     let mut highest = None;
    //     let mut count = 0;
    //     for txu in TX_POOL.iter().filter(|e| e.value().tx.signer == pk) {
    //         count += 1;
    //         let nonce = txu.value().tx.nonce;
    //         if highest.map_or(true, |h| nonce > h) {
    //             highest = Some(nonce);
    //         }
    //     }
    //     (highest, count)
    // }

    // pub fn add_gifted_sol(sol: &[u8]) -> bool {
    //     let hash = blake3::hash(sol).as_bytes().to_vec();
    //     if GIFTED_SOL_CACHE.contains_key(&hash) {
    //         false
    //     } else {
    //         GIFTED_SOL_CACHE.insert(hash, Consensus::chain_epoch());
    //         true
    //     }
    // }
}
