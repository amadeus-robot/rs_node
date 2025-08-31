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
    pub fn sign(entry_hash: [u8; 32], mutations_hash: [u8; 32]) -> Self {
        // Fetch from config or env
        let sk_bytes = CONFIG.ama.trainer_sk.clone();
        let pk_bytes = CONFIG.ama.trainer_pk().clone();

        let mut msg = Vec::new();
        msg.extend_from_slice(&entry_hash);
        msg.extend_from_slice(&mutations_hash);

        let sig = BlsRs::sign(&sk_bytes, &msg, BLS12AggSig::DST_ATT).unwrap();

        Self {
            entry_hash: entry_hash.to_vec(),
            mutations_hash: mutations_hash.to_vec(),
            signer: pk_bytes,
            signature: sig,
        }
    }

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

#[cfg(test)]
mod attestation_tests {
    use super::*;

    #[test]
    fn test_pack_unpack() {
        let att = Attestation {
            entry_hash: vec![1; 32],
            mutations_hash: vec![2; 32],
            signer: vec![3; 48],
            signature: vec![4; 96],
        };

        println!("Original Attestation: {:?}", att);

        // Pack the attestation
        let packed = att.pack();

        println!("Packed bytes: {:?}", packed);
        assert!(!packed.is_empty(), "Packed bytes should not be empty");

        // Unpack it back
        let unpacked = Attestation::unpack(&packed).expect("Unpack should succeed");
        println!("Unpacked Attestation: {:?}", unpacked);

        assert_eq!(att.entry_hash, unpacked.entry_hash);
        assert_eq!(att.mutations_hash, unpacked.mutations_hash);
        assert_eq!(att.signer, unpacked.signer);
        assert_eq!(att.signature, unpacked.signature);
    }

    #[test]
    fn test_unpack_invalid_bytes() {
        let invalid_bytes = vec![0, 1, 2, 3]; // not a valid serialized Attestation

        let result = Attestation::unpack(&invalid_bytes);
        assert!(result.is_err(), "Unpack should fail for invalid bytes");
        println!("Expected error: {:?}", result.err().unwrap());
    }

    #[test]
    fn test_attestation_field_lengths() {
        let att = Attestation {
            entry_hash: vec![0; 32],
            mutations_hash: vec![0; 32],
            signer: vec![0; 48],
            signature: vec![0; 96],
        };

        assert_eq!(att.entry_hash.len(), 32);
        assert_eq!(att.mutations_hash.len(), 32);
        assert_eq!(att.signer.len(), 48);
        assert_eq!(att.signature.len(), 96);
    }
}
