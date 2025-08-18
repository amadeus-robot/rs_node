// use blst::{PublicKey, SecretKey, Signature, sign, verify};
use serde::{Deserialize, Serialize};
use std::error::Error;

use crate::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attestation {
    pub entry_hash: Vec<u8>,
    pub mutations_hash: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
}

impl Attestation {
    /// Pack attestation into bytes
    pub fn pack(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization should not fail")
    }

    /// Unpack attestation from bytes
    pub fn unpack(packed: &[u8]) -> Result<Self, Box<dyn Error>> {
        let att: Attestation = bincode::deserialize(packed)?;
        Ok(att)
    }

    // Sign entry_hash + mutations_hash with trainer secret key
    // pub fn sign(entry_hash: [u8; 32], mutations_hash: [u8; 32]) -> Self {
    //     // Fetch from config or env
    //     let sk_bytes = std::env::var("TRAINER_SK").expect("TRAINER_SK not set");
    //     let pk_bytes = std::env::var("TRAINER_PK").expect("TRAINER_PK not set");

    //     let sk = SecretKey::from_bytes(&hex::decode(sk_bytes).unwrap()).unwrap();
    //     let pk = PublicKey::from_bytes(&hex::decode(pk_bytes).unwrap()).unwrap();

    //     let mut msg = Vec::new();
    //     msg.extend_from_slice(&entry_hash);
    //     msg.extend_from_slice(&mutations_hash);

    //     let sig: Signature = sign(&sk, &msg);

    //     Self {
    //         entry_hash,
    //         mutations_hash,
    //         signer: pk.to_bytes(),
    //         signature: sig.to_bytes(),
    //     }
    // }

    // Validate the attestation structure & signature
    // pub fn validate(&self) -> Result<(), AttestationError> {
    //     if self.entry_hash.len() != 32 {
    //         return Err(AttestationError::EntryHashInvalid);
    //     }
    //     if self.mutations_hash.len() != 32 {
    //         return Err(AttestationError::MutationsHashInvalid);
    //     }
    //     if self.signer.len() != 48 {
    //         return Err(AttestationError::SignerInvalid);
    //     }

    //     let mut msg = Vec::new();
    //     msg.extend_from_slice(&self.entry_hash);
    //     msg.extend_from_slice(&self.mutations_hash);

    //     if !verify(
    //?         &PublicKey::from_bytes(&self.signer).unwrap(),
    //?         &self.signature,
    //?         &msg,
    //     ) {
    //         return Err(AttestationError::InvalidSignature);
    //     }

    //     Ok(())
    // }

    // Validate against the chain (pseudo-code)
    // pub fn validate_vs_chain(&self) -> bool {
    //     if let Some(entry) = Fabric::entry_by_hash(&self.entry_hash) {
    //         let chain_height = Consensus::chain_height();
    //         if entry.header_unpacked.height <= chain_height {
    //             if let Some(trainers) = Consensus::trainers_for_height(entry.height()) {
    //                 return trainers.contains(&self.signer);
    //             }
    //         }
    //     }
    //     false
    // }
}

#[derive(Debug)]
pub enum AttestationError {
    EntryHashInvalid,
    MutationsHashInvalid,
    SignerInvalid,
    InvalidSignature,
}
