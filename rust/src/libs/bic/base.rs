use crate::*;
use blake3::hash;

pub struct Base;

impl Base {
    pub fn exec_cost(txu: &Txu) -> i64 {
        let bytes = txu.tx_encoded.len() as i64 + 32 + 96;
        3 + (bytes / 256) * 3
    }

    pub fn seed_random(vr: &[u8], txhash: &[u8], action_index: &str, call_cnt: &str) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(vr);
        data.extend_from_slice(txhash);
        data.extend_from_slice(action_index.as_bytes());
        data.extend_from_slice(call_cnt.as_bytes());
        hash(&data).as_bytes().to_vec()
    }

    pub fn call_txs_pre_parallel(env: MapEnv, txus: Vec<Txu>) -> (Vec<String>, Vec<String>) {
        (vec![], vec![])
    }

    // pub fn call_tx_actions(
    //     env: &mut MapEnv,
    //     txu: &Txu,
    // ) -> (
    //     Vec<Mutation>,
    //     Vec<Mutation>,
    //     Vec<Mutation>,
    //     Vec<Mutation>,
    //     (),
    //     // TxReturn,
    // ) {
    //     Process::delete("mutations_gas");
    //     Process::delete("mutations_gas_reverse");
    //     Process::delete("mutations");
    //     Process::delete("mutations_reverse");

    //     let result: Result<TxReturn, TxError> = (|| -> TxReturn {
    //         let action = txu
    //             .tx
    //             .unwrap()
    //             .actions
    //             .get(0)
    //             .ok_or(TxError::NoActions)
    //             .unwrap();

    //         env.account_current = Some(action.contract.clone());

    //         if BlsRs::validate_public_key(&action.contract.as_bytes()) {
    //             if let Some(bytecode) = Contract::bytecode(&action.contract) {
    //                 let seed_bin = Self::seed_random(
    //                     &env.entry_vr,
    //                     env.tx_hash.unwrap().as_ref(),
    //                     "0",
    //                     &env.call_counter.to_string(),
    //                 );
    //                 let float64 = f64::from_le_bytes(seed_bin[..8].try_into().unwrap());
    //                 env.seed = Some(seed_bin);
    //                 env.seedf64 = float64;

    //                 if !action.attached_symbol.unwrap().is_empty() {
    //                     env.attached_symbol = action.attached_symbol.unwrap();
    //                     env.attached_amount = action.attached_amount.unwrap();

    //                     let amount: i64 = action.attached_amount.unwrap();
    //                     if amount <= 0 {
    //                         return TxResult::Err(TxError::InvalidAttachedAmount).expect("REASON");
    //                     }
    //                     if amount
    //                         > Coin::balance(
    //                             env.tx_signer.as_ref().unwrap(),
    //                             &action.attached_symbol.unwrap(),
    //                         )
    //                     {
    //                         return TxResult::Err(TxError::AttachedAmountInsufficientFunds)
    //                             .expect("REASON");
    //                     }

    //                     ConsensusKV::kv_increment(
    //                         format!(
    //                             "bic:coin:balance:{}:{}",
    //                             action.contract,
    //                             action.attached_symbol.unwrap(),
    //                         )
    //                         .as_bytes()
    //                         .to_vec(),
    //                         amount,
    //                     );
    //                     ConsensusKV::kv_increment(
    //                         format!(
    //                             "bic:coin:balance:{}:{}",
    //                             env.tx_signer.as_ref().unwrap(),
    //                             action.attached_symbol.unwrap(),
    //                         )
    //                         .as_bytes()
    //                         .to_vec(),
    //                         -amount,
    //                     );
    //                 }

    //                 let string_args: Vec<String> = action
    //                     .args
    //                     .iter()
    //                     .map(|v| String::from_utf8(v.clone()).unwrap()) // or handle error properly
    //                     .collect();

    //                 let result = WASM::call(
    //                     env.clone(),      // MapEnv
    //                     &bytecode,        // &[u8]
    //                     &action.function, // &str
    //                     &string_args,     // &[String]
    //                 ).unwrap();

    //                 let muts = Process::get("mutations").unwrap_or_default();
    //                 Process::delete("mutations");
    //                 let muts_rev = Process::get("mutations_reverse").unwrap_or_default();
    //                 Process::delete("mutations_reverse");

    //                 let exec_used = result.exec_used.unwrap_or(0) * 100;
    //                 ConsensusKV::kv_increment(
    //                     format!(
    //                         "bic:coin:balance:{}:AMA",
    //                         String::from_utf8(env.entry_signer).unwrap()
    //                     )
    //                     .as_bytes()
    //                     .to_vec(),
    //                     exec_used,
    //                 );
    //                 ConsensusKV::kv_increment(
    //                     format!("bic:coin:balance:{}:AMA", env.tx_signer.as_ref().unwrap())
    //                         .as_bytes()
    //                         .to_vec(),
    //                     -exec_used,
    //                 );

    //                 Process::put(
    //                     "mutations_gas",
    //                     Process::get("mutations").unwrap_or_default(),
    //                 );
    //                 Process::put(
    //                     "mutations_gas_reverse",
    //                     Process::get("mutations_reverse").unwrap_or_default(),
    //                 );
    //                 Process::put("mutations", muts);
    //                 Process::put("mutations_reverse", muts_rev);

    //                 result
    //             } else {
    //                 TxResult::Err(TxError::AccountHasNoBytecode);
    //             }
    //         } else {
    //             Self::seed_random(&env.entry_vr, env.tx_hash.unwrap().as_ref(), "0", "");

    //             if !["Epoch", "Coin", "Contract"].contains(&action.contract.as_str()) {
    //                 return TxResult::Err(TxError::InvalidBic);
    //             }
    //             if ![
    //                 "submit_sol",
    //                 "transfer",
    //                 "set_emission_address",
    //                 "slash_trainer",
    //                 "deploy",
    //             ]
    //             .contains(&action.function.as_str())
    //             {
    //                 return TxResult::Err(TxError::InvalidFunction);
    //             }

    //             let contract: Box<dyn Contract> = match action.contract {
    //                 ContractType::Epoch => Box::new(Epoch),
    //                 ContractType::Coin => Box::new(Coin),
    //                 ContractType::Contract => Box::new(ContractImpl),
    //             };

    //             match contract.call(&action.function, &env, &action.args) {
    //                 Ok(_) => HashMap::from([("error", "ok")]),
    //                 Err(_) => HashMap::from([("error", "failed")]),
    //             }
    //         }
    //     })();

    //     (
    //         Process::get("mutations").unwrap_or_default(),
    //         Process::get("mutations_reverse").unwrap_or_default(),
    //         Process::get("mutations_gas").unwrap_or_default(),
    //         Process::get("mutations_gas_reverse").unwrap_or_default(),
    //         (),
    //     )
    // }
}
