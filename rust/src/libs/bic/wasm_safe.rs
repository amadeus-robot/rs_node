use std::sync::mpsc;

use crate::*;

pub struct WASM_SAFE;

impl WASM_SAFE {
    pub async fn safe_call(mapenv: MapEnv, wasmbytes: &[u8], function: &str, args: &[String]) {
        // match wasmer_ex_call(parent_tx.clone(), mapenv, wasmbytes, function, args).await {
        //     WasmerResult::Error(reason) => {
        //         // Send same tuple shape: {reason, [], 0, nil}
        //         let _ = parent_tx.send((reason, vec![], 0, None)).await;
        //     }
        //     WasmerResult::Ok => {
        //         // Do nothing (nil in Elixir)
        //     }
        // }
    }

    pub fn spawn(mapenv: MapEnv, wasmbytes: &[u8], function: &str, args: &[String]) {
        tokio::task::spawn(Self::safe_call(mapenv, wasmbytes, function, args));
    }
}
