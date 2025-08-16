use bls::{aggregate_signatures, Signature};
use bitvec::prelude::*;
use std::collections::HashMap;

use crate::*;
pub struct BLS12AggSig;

impl BLS12AggSig {
    pub const DST: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
    pub const DST_POP: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";
    pub const DST_ATT: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_ATTESTATION_";
    pub const DST_ENTRY: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_ENTRY_";
    pub const DST_VRF: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_VRF_";
    pub const DST_TX: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_TX_";
    pub const DST_MOTION: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_MOTION_";
    pub const DST_NODE: &'static [u8] = b"AMADEUS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NODE_";

    pub fn new(trainers: &[Vec<u8>], pk: Vec<u8>, signature: Signature) -> AggSig {
        let index_of_trainer = trainers.iter().position(|t| *t == pk)
            .expect("Trainer not found");
        let mut mask = bitvec![0; trainers.len()];
        mask.set(index_of_trainer, true);
        AggSig { mask, aggsig: signature }
    }

    pub fn add(agg: &AggSig, trainers: &[Vec<u8>], pk: Vec<u8>, signature: Signature) -> AggSig {
        let index_of_trainer = trainers.iter().position(|t| *t == pk)
            .expect("Trainer not found");

        if agg.mask[index_of_trainer] {
            agg.clone()
        } else {
            let mut new_mask = agg.mask.clone();
            new_mask.set(index_of_trainer, true);
            let new_aggsig = aggregate_signatures(&[agg.aggsig.clone(), signature]);
            AggSig { mask: new_mask, aggsig: new_aggsig }
        }
    }

    pub fn unmask_trainers(trainers: &[Vec<u8>], mask: &BitVec) -> Vec<Vec<u8>> {
        trainers.iter()
            .zip(mask.iter())
            .filter_map(|(pk, &bit)| if bit { Some(pk.clone()) } else { None })
            .collect()
    }

    pub fn score(trainers: &[Vec<u8>], mask: &BitVec, consensus_weights: &HashMap<Vec<u8>, f64>) -> f64 {
        let trainers_signed = Self::unmask_trainers(trainers, mask);
        let max_score = trainers.len() as f64;
        let score: f64 = trainers_signed.iter()
            .map(|pk| *consensus_weights.get(pk).unwrap_or(&0.0))
            .sum();
        score / max_score
    }
}

#[derive(Clone)]
pub struct AggSig {
    pub mask: BitVec,
    pub aggsig: Signature,
}
