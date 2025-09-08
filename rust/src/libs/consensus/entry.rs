use blake3;
use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use std::collections::HashMap;

use crate::*;

#[derive(Debug, thiserror::Error)]
pub enum EntryError {
    #[error("TEMPORARY_txs_only_100_per_entry")]
    TooManyTxs,
    #[error("prev_hash_not_256_bits")]
    PrevHashNot256Bits,
    #[error("dr_not_256_bits")]
    DrNot256Bits,
    #[error("vr_not_96_bytes")]
    VrNot96Bytes,
    #[error("signer_not_48_bytes")]
    SignerNot48Bytes,
    #[error("txs_hash_not_256_bits")]
    TxsHashNot256Bits,
    #[error("mask_not_bitstring")]
    MaskNotBitstring,
    #[error("txs_hash_invalid")]
    TxsHashInvalid,

    #[error("unknown")]
    Unknown,
}

// Entry header and entry structs
#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
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

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub struct Entry {
    pub signature: Vec<u8>,
    pub hash: Vec<u8>,
    pub header_unpacked: EntryHeader,
    pub txs: Vec<Vec<u8>>,
    pub mask: Option<Vec<u8>>, // optional mask
}

// Entry methods
impl Entry {
    pub fn unpack(entry_packed: Option<&[u8]>) -> Option<Self> {
        match entry_packed {
            None => None,
            Some(bytes) => {
                let entry: Entry = Entry::try_from_slice(bytes).ok()?;

                Some(entry)
            }
        }
    }

    pub fn pack(e: Entry) -> Vec<u8> {
        let mut writer: Vec<u8> = vec![];

        // borrow writer mutably
        Self::serialize(&e, &mut writer).unwrap();

        writer
    }

    pub fn sign(mut entry_unpacked: Entry) -> Entry {
        let sk = &CONFIG.ama.trainer_sk.clone();
        // txs_hash = hash of concatenated txs
        let txs_concat: Vec<u8> = entry_unpacked.txs.concat();
        let txs_hash = blake3::hash(&txs_concat).as_bytes().to_vec();
        entry_unpacked.header_unpacked.txs_hash = txs_hash.clone();

        let mut header_bin: Vec<u8> = vec![];

        let header_bin = to_vec(&entry_unpacked.header_unpacked).unwrap();

        let hash = blake3::hash(&header_bin).as_bytes().to_vec();
        let signature = BlsRs::sign(sk, &hash, b"BLS12AggSig_dst_entry").unwrap();

        Entry {
            header_unpacked: entry_unpacked.header_unpacked.clone(),
            txs: entry_unpacked.txs.clone(),
            hash,
            signature,
            mask: entry_unpacked.mask.clone(),
        }
    }

    pub fn validate_signature(entry_unpacked: &Entry) -> Result<(), &'static str> {
        let hash = blake3::hash(&to_vec(&entry_unpacked.header_unpacked).unwrap())
            .as_bytes()
            .to_vec();
        if let Some(mask) = &entry_unpacked.mask {
            // Placeholder for masked BLS validation
            let trainers = Consensus::trainers_for_height(entry_unpacked.header_unpacked.height);
            let _trainers_signed = trainers; // unmask logic
            let agg_pk = BlsRs::aggregate_public_keys(_trainers_signed).unwrap();
            if !BlsRs::verify(
                &agg_pk,
                &entry_unpacked.signature,
                &hash,
                b"BLS12AggSig_dst_entry",
            ) {
                return Err("invalid_signature");
            }
        } else {
            if !BlsRs::verify(
                &entry_unpacked.header_unpacked.signer,
                &entry_unpacked.signature,
                &hash,
                b"BLS12AggSig_dst_entry",
            ) {
                return Err("invalid_signature");
            }
        }
        Ok(())
    }

    pub fn validate_entry(e: &Entry) -> Result<(), EntryError> {
        let eh = &e.header_unpacked;

        if e.txs.len() > 100 {
            return Err(EntryError::TooManyTxs);
        }

        if eh.prev_hash.len() != 32 {
            return Err(EntryError::PrevHashNot256Bits);
        }
        if eh.dr.len() != 32 {
            return Err(EntryError::DrNot256Bits);
        }
        if eh.vr.len() != 96 {
            return Err(EntryError::VrNot96Bytes);
        }
        if eh.signer.len() != 48 {
            return Err(EntryError::SignerNot48Bytes);
        }
        if eh.txs_hash.len() != 32 {
            return Err(EntryError::TxsHashNot256Bits);
        }

        if let Some(mask) = &e.mask {
            if mask.is_empty() {
                return Err(EntryError::MaskNotBitstring);
            }
        }

        let mut txs_concat = Vec::new();
        for tx in &e.txs {
            txs_concat.extend_from_slice(tx);
        }
        let txs_hash = blake3::hash(&txs_concat).as_bytes().to_vec();
        if eh.txs_hash != txs_hash {
            return Err(EntryError::TxsHashInvalid);
        }

        let is_special_meeting_block = e.mask.is_some();

        for tx in &e.txs {
            let res = TX::validate(tx, is_special_meeting_block); // define TX.validate
            if let Err(err) = res {
                return Err(EntryError::Unknown); // map specific errors as needed
            }
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
        if !BlsRs::verify(&neh.signer, &neh.vr, &ceh.vr, b"BLS12AggSig_dst_vrf") {
            return Err("invalid_vr");
        }

        let mut state: HashMap<(u8, Vec<u8>), u64> = HashMap::new();
        for tx in &next_entry.txs {
            let txu = TX::unpack(tx).unwrap();
            // Validate nonce, balance, epoch, etc. Placeholder

            
        }

        Ok(())
    }

    pub fn build_next(cur_entry: &Entry, slot: u64, pk: &[u8], sk: &[u8]) -> Entry {
        let dr = blake3::hash(&cur_entry.header_unpacked.dr)
            .as_bytes()
            .to_vec();
        let vr = BlsRs::sign(sk, &cur_entry.header_unpacked.vr, b"BLS12AggSig_dst_vrf").unwrap();

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
            let txu = TX::unpack(tx);
            // Placeholder: assume action function name exists
            // return true if any tx action matches txfunction
        }
        false
    }
}
