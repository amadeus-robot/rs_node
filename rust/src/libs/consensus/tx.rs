use crate::*;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub op: String,
    pub contract: String,
    pub function: String,
    pub args: Vec<Vec<u8>>, // binaries in Elixir
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_amount: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub signer: String, // Base58PK in Elixir; keep String
    pub nonce: u128,    // nanoseconds can exceed u64
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Txu {
    // Elixir map has these top-level keys:
    //  tx (decoded), tx_encoded, hash, signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx: Option<Tx>,
    pub tx_encoded: Vec<u8>,
    pub hash: Vec<u8>,
    pub signature: Vec<u8>,
}

// =======================
// Errors (mirror Elixir atoms where possible)
// =======================
#[derive(Debug, thiserror::Error)]
pub enum TxError {
    #[error("no_actions")]
    NoActions,
    #[error("too_large")]
    TooLarge,
    #[error("tx_not_canonical")]
    TxNotCanonical,
    #[error("invalid_hash")]
    InvalidHash,
    #[error("invalid_signature")]
    InvalidSignature,
    #[error("nonce_not_integer")]
    NonceNotInteger,
    #[error("nonce_too_high")]
    NonceTooHigh,
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
    AttachedSymbolMustBeIncluded,
    #[error("unknown")]
    Unknown,
}

pub type TxResult<T> = Result<T, TxError>;

// =======================
// TX module (functions)
// =======================
pub struct TX;

impl TX {
    // // normalize_atoms(txu):
    // // Keep only tx_encoded/hash/signature and (if present) rebuild tx/actions,
    // // including attached_* only if both are present.
    // pub fn normalize_atoms(mut txu: Txu) -> Txu {
    //     let t = Txu {
    //         tx_encoded: txu.tx_encoded.clone(),
    //         hash: txu.hash.clone(),
    //         signature: txu.signature.clone(),
    //         tx: txu.tx.take().map(|orig_tx| {
    //             let actions = orig_tx
    //                 .actions
    //                 .into_iter()
    //                 .map(|a| {
    //                     let keep_attachment =
    //                         a.attached_symbol.is_some() && a.attached_amount.is_some();
    //                     Action {
    //                         op: a.op,
    //                         contract: a.contract,
    //                         function: a.function,
    //                         args: a.args,
    //                         attached_symbol: if keep_attachment {
    //                             a.attached_symbol
    //                         } else {
    //                             None
    //                         },
    //                         attached_amount: if keep_attachment {
    //                             a.attached_amount
    //                         } else {
    //                             None
    //                         },
    //                     }
    //                 })
    //                 .collect();
    //             Tx {
    //                 signer: orig_tx.signer,
    //                 nonce: orig_tx.nonce,
    //                 actions,
    //             }
    //         }),
    //     };
    //     t
    // }

    // // validate(tx_packed, is_special_meeting_block \\ false)
    // // Returns Ok(txu) on success, Err(TxError) on failure.
    // pub fn validate(env: &impl Env, tx_packed: &[u8], is_special_meeting_block: bool) -> TxResult<Txu> {
    //     // size check
    //     if tx_packed.len() >= env.tx_size() {
    //         return Err(TxError::TooLarge);
    //     }

    //     // txu = VanillaSer.decode!(tx_packed)
    //     let mut txu_raw: Txu = VanillaSer::decode(tx_packed);

    //     // Keep only tx_encoded/hash/signature then decode tx from tx_encoded
    //     let tx_encoded = txu_raw.tx_encoded.clone();
    //     let mut tx: Tx = VanillaSer::decode(&tx_encoded);

    //     // Normalize actions: keep only specific keys and Optional attachments
    //     let actions: Vec<Action> = tx
    //         .actions
    //         .into_iter()
    //         .map(|a| Action {
    //             op: a.op,
    //             contract: a.contract,
    //             function: a.function,
    //             args: a.args,
    //             attached_symbol: a.attached_symbol,
    //             attached_amount: a.attached_amount,
    //         })
    //         .collect();

    //     tx.actions = actions;
    //     txu_raw.tx = Some(tx.clone());

    //     let hash = txu_raw.hash.clone();
    //     let signature = txu_raw.signature.clone();

    //     // normalize_atoms(txu)
    //     let mut txu = TX::normalize_atoms(txu_raw);

    //     // canonical check
    //     let canonical = Canonical {
    //         tx_encoded: VanillaSer::encode(txu.tx.as_ref().unwrap()),
    //         hash: hash.clone(),
    //         signature: signature.clone(),
    //     };
    //     let canonical_bytes = VanillaSer::encode(&canonical);
    //     if tx_packed != canonical_bytes {
    //         return Err(TxError::TxNotCanonical);
    //     }

    //     // hash check
    //     if hash != blake3_hash(&txu.tx_encoded) {
    //         return Err(TxError::InvalidHash);
    //     }

    //     // signature check
    //     let tx_ref = txu.tx.as_ref().ok_or(TxError::Unknown)?;
    //     if !env.bls_verify(tx_ref.signer.as_bytes(), &txu.signature, &txu.hash) {
    //         return Err(TxError::InvalidSignature);
    //     }

    //     // nonce checks
    //     // (In Elixir: is_integer nonce; here it's typed u128 so it's integer by type)
    //     let nonce = tx_ref.nonce;
    //     if nonce > 99_999_999_999_999_999_999u128 {
    //         return Err(TxError::NonceTooHigh);
    //     }

    //     // actions checks
    //     let actions_ref = &tx_ref.actions;
    //     if actions_ref.is_empty() {
    //         return Err(TxError::ActionsMustBeList);
    //     }
    //     if actions_ref.len() != 1 {
    //         return Err(TxError::ActionsLengthMustBe1);
    //     }
    //     let action = &actions_ref[0];

    //     if action.op != "call" {
    //         return Err(TxError::OpMustBeCall);
    //     }
    //     // contract/function binary checks => here both are Strings; consider non-empty
    //     if action.contract.is_empty() {
    //         return Err(TxError::ContractMustBeBinary);
    //     }
    //     if action.function.is_empty() {
    //         return Err(TxError::FunctionMustBeBinary);
    //     }

    //     // args must be list of binaries
    //     // (already Vec<Vec<u8>> so type ensures list; ensure each arg is binary)
    //     for arg in &action.args {
    //         if arg.is_empty() {
    //             // In Elixir: "is_binary" (empty is allowed there, but we keep parity by not failing on empty)
    //             // If you want to forbid empty, uncomment next line:
    //             // return Err(TxError::ArgMustBeBinary);
    //         }
    //     }

    //     // contract/function validity
    //     let core_ok = (action.contract == "Epoch"
    //         || action.contract == "Coin"
    //         || action.contract == "Contract")
    //         && (action.function == "submit_sol"
    //             || action.function == "transfer"
    //             || action.function == "set_emission_address"
    //             || action.function == "slash_trainer"
    //             || action.function == "deploy");

    //     let is_valid_contract = core_ok || env.bls_validate_public_key(&action.contract);
    //     if !is_valid_contract {
    //         return Err(TxError::InvalidContractOrFunction);
    //     }

    //     // special meeting block rules
    //     if is_special_meeting_block {
    //         if action.contract != "Epoch" {
    //             return Err(TxError::InvalidModuleForSpecialMeeting);
    //         }
    //         if action.function != "slash_trainer" {
    //             return Err(TxError::InvalidFunctionForSpecialMeeting);
    //         }
    //     }

    //     // attachment checks
    //     if let Some(sym) = &action.attached_symbol {
    //         // "must be binary" satisfied by type; enforce size 1..32
    //         let len = sym.len();
    //         if len < 1 || len > 32 {
    //             return Err(TxError::AttachedSymbolWrongSize);
    //         }
    //     }
    //     if action.attached_symbol.is_some() && action.attached_amount.is_none() {
    //         return Err(TxError::AttachedAmountMustBeIncluded);
    //     }
    //     if action.attached_amount.is_some() && action.attached_symbol.is_none() {
    //         return Err(TxError::AttachedSymbolMustBeIncluded);
    //     }

    //     Ok(txu)
    // }

    // // build(sk, contract, function, args, nonce \\ nil, attached_symbol \\ nil, attached_amount \\ nil)
    // // -> packed bytes (like VanillaSer.encode(%{tx_encoded, hash, signature}))
    // pub fn build(
    //     env: &impl Env,
    //     sk: &[u8],
    //     contract: &str,
    //     function: &str,
    //     args: Vec<Vec<u8>>,
    //     nonce: Option<u128>,
    //     attached_symbol: Option<Vec<u8>>,
    //     attached_amount: Option<Vec<u8>>,
    // ) -> Vec<u8> {
    //     let pk = env.bls_get_public_key(sk);
    //     let nonce = nonce.unwrap_or_else(|| {
    //         use std::time::{SystemTime, UNIX_EPOCH};
    //         SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .unwrap()
    //             .as_nanos()
    //     });

    //     let mut action = Action {
    //         op: "call".into(),
    //         contract: contract.into(),
    //         function: function.into(),
    //         args,
    //         attached_symbol: None,
    //         attached_amount: None,
    //     };

    //     if attached_symbol.is_some() && attached_amount.is_some() {
    //         action.attached_symbol = attached_symbol;
    //         action.attached_amount = attached_amount;
    //     }

    //     let tx = Tx {
    //         signer: String::from_utf8_lossy(&pk).to_string(), // keep as String like Elixir Base58PK
    //         nonce,
    //         actions: vec![action],
    //     };

    //     let tx_encoded = VanillaSer::encode(&tx);
    //     let hash = blake3_hash(&tx_encoded);
    //     let signature = env.bls_sign(sk, &hash);

    //     let outer = Canonical {
    //         tx_encoded,
    //         hash,
    //         signature,
    //     };
    //     VanillaSer::encode(&outer)
    // }

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

    // pack(txu) => encode %{tx_encoded, hash, signature}
    pub fn pack(txu: &Txu) -> Vec<u8> {
        VanillaSer::encode(&Term::Binary(bincode::serialize(&txu).unwrap()))
    }

    pub fn unpack(tx_packed: &[u8]) -> Txu {
        // Decode outer Canonical structure directly
        let (term, txu): (Term, &[u8]) =
            VanillaSer::decode(tx_packed).expect("Failed to decode tx_packed");

        if let Term::Binary(inner_bytes) = term {
            bincode::deserialize::<Txu>(&inner_bytes).expect("Failed to deserialize Txu")
        } else {
            panic!("Expected a Term::Binary for Txu");
        }
    }
}
