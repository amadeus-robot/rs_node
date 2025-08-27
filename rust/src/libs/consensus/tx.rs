use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, HashMap},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub op: String,
    pub contract: String,
    pub function: String,
    pub args: Vec<Vec<u8>>, // binaries in Elixir
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_amount: Option<i64>,
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

#[derive(Debug, Clone)]
pub struct TxReturn {
    pub error: Option<String>,
    pub reason: Option<String>,
    pub exec_used: u64,
}

// =======================
// TX module (functions)
// =======================
pub struct TX;

impl TX {
    fn term_to_bytes(term: &Term) -> Result<Vec<u8>, TxError> {
        if let Term::Binary(bytes) = term {
            Ok(bytes.clone())
        } else {
            Err(TxError::InvalidTerm)
        }
    }
    // // including attached_* only if both are present.
    pub fn normalize_atoms(mut txu: HashMap<String, Term>) -> HashMap<String, Term> {
        // Extract top-level fields
        let tx_encoded = txu.remove("tx_encoded").unwrap_or(Term::Nil);
        let hash = txu.remove("hash").unwrap_or(Term::Nil);
        let signature = txu.remove("signature").unwrap_or(Term::Nil);

        let mut t = HashMap::new();
        t.insert("tx_encoded".to_string(), tx_encoded);
        t.insert("hash".to_string(), hash);
        t.insert("signature".to_string(), signature);

        // Process "tx" if present
        if let Some(Term::Map(tx_map)) = txu.remove("tx") {
            // Convert BTreeMap<Term, Term> into HashMap<String, Term>
            let tx: HashMap<String, Term> = tx_map
                .into_iter() // consume BTreeMap
                .filter_map(|(k, v)| {
                    if let Term::Atom(s) = k {
                        Some((s, v)) // move owned key & value
                    } else {
                        None
                    }
                })
                .collect();

            let signer = tx.get("signer").cloned().unwrap_or(Term::Nil);
            let nonce = tx.get("nonce").cloned().unwrap_or(Term::Nil);

            let actions = match tx.get("actions") {
                Some(Term::List(list)) => list
                    .into_iter() // consume list, own elements
                    .map(|action_term| {
                        if let Term::Map(action_map) = action_term {
                            let action: HashMap<String, Term> = action_map
                                .iter()
                                .filter_map(|(k, v)| {
                                    if let Term::Atom(s) = k {
                                        Some((s.clone(), v.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            let op = action.get("op").cloned().unwrap_or(Term::Nil);
                            let contract = action.get("contract").cloned().unwrap_or(Term::Nil);
                            let function = action.get("function").cloned().unwrap_or(Term::Nil);
                            let args = action.get("args").cloned().unwrap_or(Term::Nil);

                            let attached_symbol = action.get("attached_symbol").cloned();
                            let attached_amount = action.get("attached_amount").cloned();
                            let keep_attachment =
                                attached_symbol.is_some() && attached_amount.is_some();

                            let mut new_action = BTreeMap::new();
                            new_action.insert(Term::Atom("op".to_string()), op);
                            new_action.insert(Term::Atom("contract".to_string()), contract);
                            new_action.insert(Term::Atom("function".to_string()), function);
                            new_action.insert(Term::Atom("args".to_string()), args);

                            if keep_attachment {
                                new_action.insert(
                                    Term::Atom("attached_symbol".to_string()),
                                    attached_symbol.unwrap(),
                                );
                                new_action.insert(
                                    Term::Atom("attached_amount".to_string()),
                                    attached_amount.unwrap(),
                                );
                            }

                            Term::Map(new_action)
                        } else {
                            Term::Nil
                        }
                    })
                    .collect(),
                _ => vec![],
            };

            let mut new_tx = BTreeMap::new();
            new_tx.insert(Term::Atom("signer".to_string()), signer);
            new_tx.insert(Term::Atom("nonce".to_string()), nonce);
            new_tx.insert(Term::Atom("actions".to_string()), Term::List(actions));

            t.insert("tx".to_string(), Term::Map(new_tx));
        }

        t
    }
    // // validate(tx_packed, is_special_meeting_block \\ false)
    // // Returns Ok(txu) on success, Err(TxError) on failure.
    pub fn validate(tx_packed: &[u8], is_special_meeting_block: bool) -> TxResult<Txu> {
        let tx_size = CONFIG.ama.tx_size as usize;
        // size check
        if tx_packed.len() >= tx_size {
            return Err(TxError::TooLarge);
        }

        let txu_raw = VanillaSer::decode(tx_packed).unwrap();

        // Step 2: pick wanted top-level keys
        let wanted_keys = ["tx_encoded", "hash", "signature"];
        let mut txu: HashMap<String, Term> = HashMap::new();

        if let Term::Map(m) = txu_raw {
            for k in &wanted_keys {
                let key = Term::Binary(k.as_bytes().to_vec());
                if let Some(v) = m.get(&key) {
                    txu.insert(k.to_string(), v.clone());
                }
            }
        } else {
            return Err(TxError::Unknown);
        }

        // Step 3: fetch tx_encoded and decode
        let tx_encoded_bytes = match txu.get("tx_encoded") {
            Some(Term::Binary(b)) => b.clone(),
            _ => return Err(TxError::Unknown),
        };
        let tx_raw = VanillaSer::decode(&tx_encoded_bytes).unwrap();

        // Step 4: pick tx fields
        let tx_wanted = ["signer", "nonce", "actions"];
        let mut tx: BTreeMap<Term, Term> = BTreeMap::new();
        if let Term::Map(m) = tx_raw {
            for k in &tx_wanted {
                let key = Term::Binary(k.as_bytes().to_vec());
                if let Some(v) = m.get(&key) {
                    tx.insert(key.clone(), v.clone());
                }
            }
        } else {
            return Err(TxError::Unknown);
        }

        // Step 5: filter actions
        if let Some(Term::List(actions_list)) = tx.get(&Term::Binary(b"actions".to_vec())) {
            let action_keys = [
                "op",
                "contract",
                "function",
                "args",
                "attached_symbol",
                "attached_amount",
            ];
            let mut filtered_actions = Vec::new();

            for action in actions_list {
                if let Term::Map(a_map) = action {
                    let mut filtered_map = BTreeMap::new();
                    for key_str in &action_keys {
                        let key = Term::Binary(key_str.as_bytes().to_vec());
                        if let Some(v) = a_map.get(&key) {
                            filtered_map.insert(key.clone(), v.clone());
                        }
                    }
                    filtered_actions.push(Term::Map(filtered_map));
                }
            }

            tx.insert(
                Term::Binary(b"actions".to_vec()),
                Term::List(filtered_actions),
            );
        }

        // Step 6: insert tx into txu
        txu.insert("tx".to_string(), Term::Map(tx.clone()));

        // Step 7: fetch hash and signature
        let hash = txu.get("hash").ok_or("missing hash").unwrap();
        let signature = txu.get("signature").ok_or("missing signature").unwrap();

        let txu = Self::normalize_atoms(txu);

        let mut inner_map = HashMap::new();
        inner_map.insert(
            "tx_encoded".to_string(),
            Term::Binary(VanillaSer::encode(&Term::Map(tx.clone()))),
        );
        inner_map.insert("hash".to_string(), hash.clone()); // Term::Binary or whatever type hash is
        inner_map.insert("signature".to_string(), signature.clone());

        let mut btree = BTreeMap::new();
        for (k, v) in inner_map.into_iter() {
            btree.insert(Term::Atom(k), v); // key must be Term::Atom
        }

        // Wrap as Term::Map
        let canonical_term = Term::Map(btree);

        // Encode using VanillaSer
        let canonical: Vec<u8> = VanillaSer::encode(&canonical_term);

        if tx_packed != canonical.as_slice() {
            return Err(TxError::TxNotCanonical);
        };

        if let Term::Binary(ref hash_bytes) = hash {
            let tx_btree = {
                let mut map = BTreeMap::new();
                for (k, v) in tx.clone().into_iter() {
                    // k: Term, v: Term
                    map.insert(k, v); // insert directly
                }
                Term::Map(map)
            };

            let computed_hash = blake3::hash(&VanillaSer::encode(&tx_btree));

            if hash_bytes != computed_hash.as_bytes() {
                return Err(TxError::InvalidHash);
            }
        } else {
            return Err(TxError::InvalidHash);
        };

        // Extract signer_bytes, signature, and hash as &[u8]
        let signer_bytes: &[u8] = if let Some(Term::Map(tx_map)) = txu.get("tx") {
            if let Some(Term::Binary(bytes)) = tx_map.get(&Term::Atom("signer".to_string())) {
                bytes.as_slice()
            } else {
                return Err(TxError::InvalidSignerType);
            }
        } else {
            return Err(TxError::MissingTx);
        };

        let sig_bytes: &[u8] = if let Term::Binary(bytes) = &signature {
            bytes.as_slice()
        } else {
            return Err(TxError::InvalidSignature);
        };

        let msg_bytes: &[u8] = if let Term::Binary(bytes) = &hash {
            bytes.as_slice()
        } else {
            return Err(TxError::InvalidHash);
        };

        // Verify signature
        if !BlsRs::verify(signer_bytes, sig_bytes, msg_bytes, BLS12AggSig::DST_TX) {
            return Err(TxError::InvalidSignature);
        };

        let tx_map = match txu.get("tx") {
            Some(Term::Map(map)) => map,
            _ => return Err(TxError::ActionsMustBeList), // or another error
        };

        // Validate nonce
        let nonce = match tx_map.get(&Term::Atom("nonce".to_string())) {
            Some(Term::Int(n)) => *n,
            _ => return Err(TxError::NonceNotInteger),
        };

        if nonce > 99_999_999_999_999_999_999 {
            return Err(TxError::NonceTooHigh);
        }

        // Validate actions
        let actions = match tx_map.get(&Term::Atom("actions".to_string())) {
            Some(Term::List(list)) => list,
            _ => return Err(TxError::ActionsMustBeList),
        };

        if actions.len() != 1 {
            return Err(TxError::ActionsLengthMustBe1);
        }

        // Get first action
        let action = &actions[0];

        let action_map = match action {
            Term::Map(map) => map,
            _ => return Err(TxError::OpMustBeCall), // fallback
        };

        // Helper to get string fields from Term::Binary
        let get_str = |key: &str| -> Result<&Vec<u8>, TxError> {
            match action_map.get(&Term::Atom(key.to_string())) {
                Some(Term::Binary(bytes)) => Ok(bytes),
                _ => Err(match key {
                    "contract" => TxError::ContractMustBeBinary,
                    "function" => TxError::FunctionMustBeBinary,
                    _ => TxError::OpMustBeCall,
                }),
            }
        };

        // Validate op
        match action_map.get(&Term::Atom("op".to_string())) {
            Some(Term::Binary(op_bytes)) => {
                if op_bytes != b"call" {
                    return Err(TxError::OpMustBeCall);
                }
            }
            _ => return Err(TxError::OpMustBeCall),
        };


        // // signature check
        // let tx_ref = txu.tx.as_ref().ok_or(TxError::Unknown)?;
        // if !env.bls_verify(tx_ref.signer.as_bytes(), &txu.signature, &txu.hash) {
        //     return Err(TxError::InvalidSignature);
        // }

        // // nonce checks
        // // (In Elixir: is_integer nonce; here it's typed u128 so it's integer by type)
        // let nonce = tx_ref.nonce;
        // if nonce > 99_999_999_999_999_999_999u128 {
        //     return Err(TxError::NonceTooHigh);
        // }

        // // actions checks
        // let actions_ref = &tx_ref.actions;
        // if actions_ref.is_empty() {
        //     return Err(TxError::ActionsMustBeList);
        // }
        // if actions_ref.len() != 1 {
        //     return Err(TxError::ActionsLengthMustBe1);
        // }
        // let action = &actions_ref[0];

        // if action.op != "call" {
        //     return Err(TxError::OpMustBeCall);
        // }
        // // contract/function binary checks => here both are Strings; consider non-empty
        // if action.contract.is_empty() {
        //     return Err(TxError::ContractMustBeBinary);
        // }
        // if action.function.is_empty() {
        //     return Err(TxError::FunctionMustBeBinary);
        // }

        // // args must be list of binaries
        // // (already Vec<Vec<u8>> so type ensures list; ensure each arg is binary)
        // for arg in &action.args {
        //     if arg.is_empty() {
        //         // In Elixir: "is_binary" (empty is allowed there, but we keep parity by not failing on empty)
        //         // If you want to forbid empty, uncomment next line:
        //         // return Err(TxError::ArgMustBeBinary);
        //     }
        // }

        // // contract/function validity
        // let core_ok = (action.contract == "Epoch"
        //     || action.contract == "Coin"
        //     || action.contract == "Contract")
        //     && (action.function == "submit_sol"
        //         || action.function == "transfer"
        //         || action.function == "set_emission_address"
        //         || action.function == "slash_trainer"
        //         || action.function == "deploy");

        // let is_valid_contract = core_ok || env.bls_validate_public_key(&action.contract);
        // if !is_valid_contract {
        //     return Err(TxError::InvalidContractOrFunction);
        // }

        // // special meeting block rules
        // if is_special_meeting_block {
        //     if action.contract != "Epoch" {
        //         return Err(TxError::InvalidModuleForSpecialMeeting);
        //     }
        //     if action.function != "slash_trainer" {
        //         return Err(TxError::InvalidFunctionForSpecialMeeting);
        //     }
        // }

        // // attachment checks
        // if let Some(sym) = &action.attached_symbol {
        //     // "must be binary" satisfied by type; enforce size 1..32
        //     let len = sym.len();
        //     if len < 1 || len > 32 {
        //         return Err(TxError::AttachedSymbolWrongSize);
        //     }
        // }
        // if action.attached_symbol.is_some() && action.attached_amount.is_none() {
        //     return Err(TxError::AttachedAmountMustBeIncluded);
        // }
        // if action.attached_amount.is_some() && action.attached_symbol.is_none() {
        //     return Err(TxError::AttachedSymbolMustBeIncluded);
        // }

        // Ok(txu)
    }

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
