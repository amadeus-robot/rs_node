use std::sync::mpsc;

use crate::*;

pub struct WASM_SAFE;

impl WASM_SAFE {
    pub async fn safe_call(
        mapenv: MapEnv,
        wasmbytes: Vec<u8>,
        function: String,
        args: Vec<String>,
    ) {
        // WasmerRs::call(mapenv, wasmbytes, function, args);
        // match wasmer_ex_call(parent_tx.clone(), mapenv, &wasmbytes, &function, &args).await {
        //     WasmerResult::Error(reason) => {
        //         let _ = parent_tx.send((reason, vec![], 0, None)).await;
        //     }
        //     WasmerResult::Ok => {
        //         // Do nothing
        //     }
        // }
    }

    pub fn spawn(mapenv: MapEnv, wasmbytes: Vec<u8>, function: String, args: Vec<String>) {
        tokio::spawn(Self::safe_call(mapenv, wasmbytes, function, args));
    }
}