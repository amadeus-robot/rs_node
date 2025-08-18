use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub mod computor_gen_type;
pub use computor_gen_type::*;
use tokio::time::sleep;

use crate::*;

struct ComputorGen {
    state: Arc<Mutex<ComputorState>>,
}

impl ComputorGen {
    fn new() -> Self {
        let state = ComputorState {
            enabled: false,
            computor_type: ComputorType::None,
        };
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    async fn start(&self, ctype: Option<ComputorType>) {
        let mut state = self.state.lock().unwrap();
        state.enabled = true;
        state.computor_type = ctype.unwrap_or(ComputorType::None);
    }

    async fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        state.enabled = false;
    }

    async fn init(&self, config_computor_type: ComputorType) {
        // Schedule the tick loop
        let state_clone = self.state.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(1)).await;
                // Here you can call self.tick(&mut state) or similar
                let state = state_clone.lock().unwrap();
                println!("tick! state: {:?}", *state);
            }
        });

        // Start computor based on config
        match config_computor_type {
            ComputorType::Trainer => self.start(Some(ComputorType::Trainer)).await,
            ComputorType::Default => self.start(None).await,
            _ => {}
        }
    }

    // async fn tick_loop(&self) {
    //     loop {
    //         sleep(Duration::from_secs(1)).await;
    //         let mut state = self.state.lock().unwrap();
    //         if !state.enabled {
    //             continue;
    //         }

    //         if !FabricSyncAttestGen::is_quorum_in_epoch().await {
    //             println!("🔴 cannot compute: out_of_sync");
    //             continue;
    //         }

    //         self.tick(&mut state).await;
    //     }
    // }

    // async fn tick(&self, state: &mut ComputorState) {
    //     println!("computor running {}", Utc::now());

    //     let pk = AppConfig::trainer_pk();
    //     let pop = AppConfig::trainer_pop();

    //     let coins = Consensus::chain_balance(&pk).await;
    //     let epoch = Consensus::chain_epoch().await;
    //     let has_exec_coins = coins >= BIC::Coin::to_cents(100);

    //     match state.computor_type {
    //         ComputorType::Trainer
    //             if !has_exec_coins || state.computor_type == ComputorType::None =>
    //         {
    //             if let Some(sol) = UPOW::compute_for(
    //                 epoch,
    //                 EntryGenesis::signer(),
    //                 EntryGenesis::pop(),
    //                 &pk,
    //                 UPOW::rand_bytes(96),
    //                 100,
    //             )
    //             .await
    //             {
    //                 println!("🔢 tensor matmul complete! broadcasting sol..");
    //                 NodeGen::broadcast("sol", "trainers", &[sol]).await;
    //             }
    //         }
    //         _ => {
    //             if let Some(sol) =
    //                 UPOW::compute_for(epoch, &pk, &pop, &pk, UPOW::rand_bytes(96), 100).await
    //             {
    //                 let sk = AppConfig::trainer_sk();
    //                 let packed_tx = TX::build(&sk, "Epoch", "submit_sol", &[sol.clone()]).await;
    //                 println!(
    //                     "🔢 tensor matmul complete! tx {}",
    //                     Base58::encode(&packed_tx.hash)
    //                 );

    //                 TXPool::insert(packed_tx.clone()).await;
    //                 NodeGen::broadcast("txpool", "trainers", &[vec![packed_tx]]).await;
    //             }
    //         }
    //     }
    // }

    // async fn set_emission_address(&self, to_address: &str) {
    //     let sk = AppConfig::trainer_sk();
    //     let packed_tx = TX::build(
    //         &sk,
    //         "Epoch",
    //         "set_emission_address",
    //         &[to_address.to_string()],
    //     )
    //     .await;
    //     TXPool::insert(packed_tx.clone()).await;
    //     NodeGen::broadcast("txpool", "trainers", &[vec![packed_tx]]).await;
    // }
}
