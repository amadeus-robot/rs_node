use blake3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::*;

// Placeholder BLS functions
mod bls {
    pub fn sign(_sk: &[u8], msg: &[u8], _dst: &[u8]) -> Vec<u8> {
        msg.to_vec() // replace with actual BLS signing
    }
    pub fn verify(_pk: &[u8], _sig: &[u8], _msg: &[u8], _dst: &[u8]) -> bool {
        true
    }
    pub fn aggregate_public_keys(_keys: Vec<&[u8]>) -> Vec<u8> {
        vec![]
    }
}

// Placeholder TX module
mod tx {
    pub fn unpack(tx: &[u8]) -> TxUnpacked {
        TxUnpacked { tx: Tx {} }
    }
    pub fn validate(_tx: &[u8], _special: bool) -> Result<(), &'static str> {
        Ok(())
    }
    pub struct Tx;
    pub struct TxUnpacked {
        pub tx: Tx,
    }
}

// Placeholder Consensus module
mod consensus {
    pub fn trainers_for_height(_height: u64) -> Vec<Vec<u8>> {
        vec![]
    }
    pub fn chain_epoch() -> u32 {
        0
    }
    pub fn chain_nonce(_signer: &[u8]) -> Option<u64> {
        Some(0)
    }
    pub fn chain_balance(_signer: &[u8]) -> i64 {
        1000
    }
}

// Entry header and entry structs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryHeader {
    pub slot: u64,
    pub height: u64,
    pub prev_slot: i64,
    pub prev_hash: Vec<u8>,
    pub signer: Vec<u8>,
    pub dr: Vec<u8>,
    pub vr: Vec<u8>,
    pub txs_hash: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub signature: Vec<u8>,
    pub hash: Vec<u8>,
    pub header_unpacked: EntryHeader,
    pub txs: Vec<Vec<u8>>,
    pub mask: Option<Vec<u8>>, // optional mask
}

// Entry methods
impl Entry {
    pub fn unpack(entry_packed: &[u8]) -> Entry {
        // Placeholder: In Elixir, binary_to_term; here use bincode, serde, or manual deserialization
        bincode::deserialize(entry_packed).unwrap()
    }

    pub fn pack(&self) -> Vec<u8> {
        // Only take relevant fields
        bincode::serialize(&self).unwrap()
    }

    pub fn sign(mut self, sk: &[u8]) -> Entry {
        // txs_hash = hash of concatenated txs
        let txs_concat: Vec<u8> = self.txs.concat();
        let txs_hash = blake3::hash(&txs_concat).as_bytes().to_vec();
        self.header_unpacked.txs_hash = txs_hash.clone();

        let header_bin = bincode::serialize(&self.header_unpacked).unwrap();
        let hash = blake3::hash(&header_bin).as_bytes().to_vec();
        let signature = bls::sign(sk, &hash, b"BLS12AggSig_dst_entry");

        Entry {
            header_unpacked: self.header_unpacked.clone(),
            txs: self.txs.clone(),
            hash,
            signature,
            mask: self.mask.clone(),
        }
    }

    pub fn validate_signature(&self) -> Result<(), &'static str> {
        let hash = blake3::hash(&bincode::serialize(&self.header_unpacked).unwrap())
            .as_bytes()
            .to_vec();
        if let Some(mask) = &self.mask {
            // Placeholder for masked BLS validation
            let trainers = consensus::trainers_for_height(self.header_unpacked.height);
            let _trainers_signed = trainers; // unmask logic
            let agg_pk =
                bls::aggregate_public_keys(_trainers_signed.iter().map(|x| x.as_slice()).collect());
            if !bls::verify(&agg_pk, &self.signature, &hash, b"BLS12AggSig_dst_entry") {
                return Err("invalid_signature");
            }
        } else {
            if !bls::verify(
                &self.header_unpacked.signer,
                &self.signature,
                &hash,
                b"BLS12AggSig_dst_entry",
            ) {
                return Err("invalid_signature");
            }
        }
        Ok(())
    }

    pub fn validate_entry(&self) -> Result<(), &'static str> {
        let eh = &self.header_unpacked;

        if eh.txs_hash != blake3::hash(&self.txs.concat()).as_bytes() {
            return Err("txs_hash_invalid");
        }
        if self.txs.len() > 100 {
            return Err("too_many_txs");
        }

        for tx in &self.txs {
            tx::validate(tx, self.mask.is_some())?;
        }
        Ok(())
    }

    pub fn validate_next(cur_entry: &Entry, next_entry: &Entry) -> Result<(), &'static str> {
        let ceh = &cur_entry.header_unpacked;
        let neh = &next_entry.header_unpacked;

        if ceh.slot != neh.prev_slot as u64 {
            return Err("invalid_slot");
        }
        if ceh.height != neh.height - 1 {
            return Err("invalid_height");
        }
        if cur_entry.hash != neh.prev_hash {
            return Err("invalid_hash");
        }
        if blake3::hash(&ceh.dr).as_bytes().to_vec() != neh.dr {
            return Err("invalid_dr");
        }
        if !bls::verify(&neh.signer, &neh.vr, &ceh.vr, b"BLS12AggSig_dst_vrf") {
            return Err("invalid_vr");
        }

        let mut state: HashMap<(u8, Vec<u8>), u64> = HashMap::new();
        for tx in &next_entry.txs {
            let txu = tx::unpack(tx);
            // Validate nonce, balance, epoch, etc. Placeholder
        }

        Ok(())
    }

    pub fn build_next(cur_entry: &Entry, slot: u64, pk: &[u8], sk: &[u8]) -> Entry {
        let dr = blake3::hash(&cur_entry.header_unpacked.dr)
            .as_bytes()
            .to_vec();
        let vr = bls::sign(sk, &cur_entry.header_unpacked.vr, b"BLS12AggSig_dst_vrf");

        let header = EntryHeader {
            slot,
            height: cur_entry.header_unpacked.height + 1,
            prev_slot: cur_entry.header_unpacked.slot as i64,
            prev_hash: cur_entry.hash.clone(),
            dr,
            vr,
            signer: pk.to_vec(),
            txs_hash: vec![],
        };

        Entry {
            header_unpacked: header,
            txs: vec![],
            hash: vec![],
            signature: vec![],
            mask: None,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.header_unpacked.height / 100_000
    }

    pub fn height(&self) -> u64 {
        self.header_unpacked.height
    }

    pub fn contains_tx(&self, txfunction: &str) -> bool {
        for tx in &self.txs {
            let txu = tx::unpack(tx);
            // Placeholder: assume action function name exists
            // return true if any tx action matches txfunction
        }
        false
    }
}
