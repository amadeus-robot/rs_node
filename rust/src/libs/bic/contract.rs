use std::collections::HashMap;

use crate::MapEnv;

// use wasmer_ex::validate_contract;

pub struct Contract;

impl Contract {
    // pub fn validate(wasmbytes: &[u8], env: MapEnv) -> HashMap<String, String> {
    //     let mut map_env = HashMap::new();

    //     // Convert env struct into map for WasmerEx
    //     // (in real code, you’d serialize fields properly)
    //     map_env.insert("readonly".to_string(), vec![env.readonly as u8]);

    //     match WasmerEx::validate_contract(&map_env, wasmbytes) {
    //         Ok(_) => {
    //             let mut res = HashMap::new();
    //             res.insert("error".to_string(), "ok".to_string());
    //             res
    //         }
    //         Err(reason) => {
    //             let mut res = HashMap::new();
    //             res.insert("error".to_string(), "abort".to_string());
    //             res.insert("reason".to_string(), reason);
    //             res
    //         }
    //     }
    // }

    // pub fn bytecode(account: &str) -> Option<Vec<u8>> {
    //     let key = format!("bic:contract:account:{}:bytecode", account);
    //     kv_get(&key)
    // }

    // pub fn call_deploy(env: &MapEnv, wasmbytes: &[u8]) {
    //     let key = format!(
    //         "bic:contract:account:{:?}:bytecode",
    //         hex::encode(&env.account_caller)
    //     );
    //     kv_put(&key, wasmbytes);
    // }
}
