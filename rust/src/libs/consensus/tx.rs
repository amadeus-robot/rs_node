use crate::*;
use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct Action {
    pub op: String,
    pub contract: String,
    pub function: String,
    pub args: Vec<Vec<u8>>, // binaries in Elixir
    pub attached_symbol: Option<String>,
    pub attached_amount: Option<u64>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Tx {
    pub signer: Vec<u8>,
    pub nonce: u128,
    pub actions: Vec<Action>,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct Txu {
    pub tx: Tx,
    pub hash: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum TxError {
    #[error("no_actions")]
    NoActions,
    #[error("too_large")]
    TooLarge,
    #[error("tx_not_canonical")]
    TxNotCanonical,
    #[error("invalid_bic")]
    InvalidBic,
    #[error("invalid_term")]
    InvalidTerm,
    #[error("invalid_function")]
    InvalidFunction,
    #[error("invalid_hash")]
    InvalidHash,
    #[error("invalid_signature")]
    InvalidSignature,
    #[error("missing_tx")]
    MissingTx,
    #[error("missing_signer")]
    MissingSigner,
    #[error("missing_signer_type")]
    InvalidSignerType,
    #[error("nonce_not_integer")]
    NonceNotInteger,
    #[error("nonce_too_high")]
    NonceTooHigh,
    #[error("account_has_no_bytecode")]
    AccountHasNoBytecode,
    #[error("actions_must_be_list")]
    ActionsMustBeList,
    #[error("actions_length_must_be_1")]
    ActionsLengthMustBe1,
    #[error("op_must_be_call")]
    OpMustBeCall,
    #[error("contract_must_be_binary")]
    ContractMustBeBinary,
    #[error("function_must_be_binary")]
    FunctionMustBeBinary,
    #[error("args_must_be_list")]
    ArgsMustBeList,
    #[error("arg_must_be_binary")]
    ArgMustBeBinary,
    #[error("invalid_attached_amount")]
    InvalidAttachedAmount,
    #[error("invalid_contract_or_function")]
    InvalidContractOrFunction,
    #[error("invalid_module_for_special_meeting")]
    InvalidModuleForSpecialMeeting,
    #[error("invalid_function_for_special_meeting")]
    InvalidFunctionForSpecialMeeting,
    #[error("attached_symbol_must_be_binary")]
    AttachedSymbolMustBeBinary,
    #[error("attached_symbol_wrong_size")]
    AttachedSymbolWrongSize,
    #[error("attached_amount_must_be_binary")]
    AttachedAmountMustBeBinary,
    #[error("attached_amount_must_be_included")]
    AttachedAmountMustBeIncluded,
    #[error("attached_symbol_must_be_included")]
    AttachedAmountInsufficientFunds,
    #[error("attached_amount_insufficient_funds")]
    AttachedSymbolMustBeIncluded,
    #[error("unknown")]
    Unknown,
}

pub type TxResult<T> = Result<T, TxError>;

pub struct TX;

impl TX {
    pub fn unpack(tx_packed: &[u8]) -> TxResult<Txu> {
        let txu = Txu::try_from_slice(tx_packed).unwrap();

        Ok(txu)
    }
    pub fn validate(tx_packed: &[u8], is_special_meeting_block: bool) -> TxResult<Txu> {
        let tx_size = CONFIG.ama.tx_size as usize;
        // size check
        if tx_packed.len() >= tx_size {
            return Err(TxError::TooLarge);
        }

        let txu = Txu::try_from_slice(tx_packed).unwrap();

        let tx = &txu.tx;
        let hash = &txu.hash;
        let signature = &txu.signature;
        let tx_encoded = to_vec(tx).unwrap();
        let actions = &tx.actions;

        let canonical_txu = &Txu {
            hash: hash.to_vec(),
            signature: signature.to_vec(),
            tx: Tx {
                actions: actions.to_vec(),
                nonce: tx.nonce,
                signer: tx.signer.to_vec(),
            },
        };

        let canonical = to_vec(canonical_txu).unwrap();

        if tx_packed != canonical {
            return Err(TxError::TxNotCanonical);
        }

        if *hash != blake3::hash(&tx_encoded).as_bytes().to_vec() {
            return Err(TxError::InvalidHash);
        }

        if !BlsRs::verify(&tx.signer, &signature.clone(), &hash, BLS12AggSig::DST_TX) {
            return Err(TxError::InvalidSignature);
        }

        let action = actions.first().unwrap();

        if action.op != "call" {
            return Err(TxError::OpMustBeCall);
        }

        let epoch = Consensus::chain_epoch();

        let contracts = ["Epoch", "Coin", "Contract"];
        let functions = [
            "submit_sol",
            "transfer",
            "set_emission_address",
            "slash_trainer",
            "deploy",
        ];

        // Check the conditions
        if contracts.contains(&action.contract.as_str())
            && functions.contains(&action.function.as_str())
        {
            // Both contract and function are allowed
        } else if BlsRs::validate_public_key(action.contract.as_bytes()) {
            // Valid public key
        } else {
            // None matched: return an error
            return Err(TxError::InvalidContractOrFunction);
        }

        if is_special_meeting_block {
            if !["Epoch"].contains(&action.contract.as_str()) {
                return Err(TxError::InvalidModuleForSpecialMeeting);
            }
            if !["slash_trainer"].contains(&action.function.as_str()) {
                return Err(TxError::InvalidModuleForSpecialMeeting);
            }
        }

        if let Some(symbol) = &action.attached_symbol {
            let len = symbol.as_bytes().len();
            if len < 1 || len > 32 {
                return Err(TxError::AttachedSymbolWrongSize);
            }
        }

        if action.attached_symbol.is_some() && action.attached_amount.is_none() {
            return Err(TxError::AttachedAmountMustBeIncluded);
        }

        if action.attached_amount.is_some() && action.attached_symbol.is_none() {
            return Err(TxError::AttachedSymbolMustBeIncluded);
        }

        Ok(txu)
    }

    pub fn build(
        sk: &[u8],
        contract: &str,
        function: &str,
        args: Vec<Vec<u8>>,
        nonce: Option<u128>,
        attached_symbol: Option<String>,
        attached_amount: Option<u64>,
    ) -> Vec<u8> {
        let pk = BlsRs::get_public_key(sk).unwrap();
        let nonce = nonce.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        });

        let mut action = Action {
            op: "call".into(),
            contract: contract.into(),
            function: function.into(),
            args,
            attached_symbol: None,
            attached_amount: None,
        };

        if attached_symbol.is_some() && attached_amount.is_some() {
            action.attached_symbol = attached_symbol;
            action.attached_amount = attached_amount;
        }

        let tx: Tx = Tx {
            signer: pk,
            nonce,
            actions: vec![action],
        };

        let tx_encoded = to_vec(&tx).unwrap();
        let hash = blake3::hash(&tx_encoded);
        let signature = BlsRs::sign(sk, hash.as_bytes(), BLS12AggSig::DST_TX).unwrap();

        let tx_built = Txu {
            hash: hash.as_bytes().to_vec(),
            signature,
            tx,
        };

        to_vec(&tx_built).unwrap()
    }

    // // chain_valid(tx_packed) and chain_valid(txu)
    // pub fn chain_valid_packed(env: &impl Env, tx_packed: &[u8]) -> bool {
    //     let txu = TX::unpack(tx_packed);
    //     TX::chain_valid(env, &txu)
    // }

    // pub fn chain_valid(env: &impl Env, txu: &Txu) -> bool {
    //     // Once more than 1 tx per entry is allowed, revisit this
    //     let Some(tx) = &txu.tx else { return false; };

    //     // nonce rules
    //     let chain_nonce = env.chain_nonce(&tx.signer);
    //     let nonce_valid = chain_nonce.map_or(true, |n| tx.nonce > n);

    //     // balance rules
    //     let has_balance = env.exec_cost(txu) <= env.chain_balance(&tx.signer);

    //     // submit_sol argument check: epoch in first 4 bytes (little-endian)
    //     let has_sol = tx.actions.iter().find_map(|a| {
    //         if a.function == "submit_sol" {
    //             a.args.first().cloned()
    //         } else {
    //             None
    //         }
    //     });

    //     let epoch_sol_valid = if let Some(arg0) = has_sol {
    //         if arg0.len() < 4 {
    //             false
    //         } else {
    //             let sol_epoch = u32::from_le_bytes([arg0[0], arg0[1], arg0[2], arg0[3]]);
    //             env.chain_epoch() == sol_epoch
    //         }
    //     } else {
    //         true
    //     };

    //     epoch_sol_valid && nonce_valid && has_balance
    // }

    // // valid_pk(pk)
    // pub fn valid_pk(env: &impl Env, pk: &str) -> bool {
    //     pk == env.burn_address() || env.bls_validate_public_key(pk)
    // }

    // // known_receivers(txu)
    // pub fn known_receivers(env: &impl Env, txu: &Txu) -> Vec<String> {
    //     let Some(tx) = &txu.tx else { return vec![]; };
    //     let a = &tx.actions[0];
    //     let c = a.contract.as_str();
    //     let f = a.function.as_str();
    //     let args = &a.args;

    //     match (c, f) {
    //         ("Coin", "transfer") => {
    //             // Variants:
    //             // [receiver, amount]
    //             // ["AMA", receiver, amount]
    //             // [receiver, amount, symbol]
    //             if args.len() >= 2 {
    //                 if args.len() == 2 {
    //                     // [receiver, _amount]
    //                     let receiver = String::from_utf8_lossy(&args[0]).to_string();
    //                     if TX::valid_pk(env, &receiver) {
    //                         return vec![receiver];
    //                     }
    //                 } else if args.len() == 3 {
    //                     // either ["AMA", receiver, _amount] OR [receiver, _amount, _symbol]
    //                     let first = String::from_utf8_lossy(&args[0]).to_string();
    //                     if first == "AMA" {
    //                         let receiver = String::from_utf8_lossy(&args[1]).to_string();
    //                         if TX::valid_pk(env, &receiver) {
    //                             return vec![receiver];
    //                         }
    //                     } else {
    //                         let receiver = first;
    //                         if TX::valid_pk(env, &receiver) {
    //                             return vec![receiver];
    //                         }
    //                     }
    //                 }
    //             }
    //             vec![]
    //         }
    //         ("Epoch", "slash_trainer") => {
    //             // [_epoch, malicious_pk, _signature, _mask_size, _mask]
    //             if args.len() >= 2 {
    //                 let malicious_pk = String::from_utf8_lossy(&args[1]).to_string();
    //                 if TX::valid_pk(env, &malicious_pk) {
    //                     return vec![malicious_pk];
    //                 }
    //             }
    //             vec![]
    //         }
    //         _ => vec![],
    //     }
    // }
}
