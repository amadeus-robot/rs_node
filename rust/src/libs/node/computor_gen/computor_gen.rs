use std::sync::Arc;
use std::time::Duration;
use futures_util::lock::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::sleep;

use crate::*;

#[derive(Debug)]
pub struct ComputorState {
    enabled: bool,
    ctype: Option<String>,
}

#[derive(Debug)]
pub enum ComputorMessage {
    Start(Option<String>),
    Stop,
    Tick,
}

pub struct ComputorGen {
    state: ComputorState,
    sender: UnboundedSender<ComputorMessage>,
    receiver: UnboundedReceiver<ComputorMessage>,
}

impl ComputorGen {
    pub fn start_link() -> Self {
        let (sender, mut receiver) = unbounded_channel();
        let state = Arc::new(Mutex::new(ComputorState {
            enabled: false,
            ctype: None,
        }));

        let state_clone = state.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            // Initial tick after 1 second
            let s_clone = sender_clone.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                let _ = s_clone.send(ComputorMessage::Tick);
            });

            while let Some(msg) = receiver.recv().await {
                let mut st = state_clone.lock().await;
                match msg {
                    ComputorMessage::Start(t) => {
                        st.enabled = true;
                        st.ctype = t;
                    }
                    ComputorMessage::Stop => {
                        st.enabled = false;
                    }
                    ComputorMessage::Tick => {
                        drop(st); // unlock before calling async tick
                        ComputorGen::handle_tick().await;
                    }
                }
            }
        });

        ComputorGen { state, sender }
    }

    pub fn sender(&self) -> UnboundedSender<ComputorMessage> {
        self.sender.clone()
    }

    async fn run(&mut self) {
        // Initial tick after 1 second
        let sender_clone = self.sender.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(1)).await;
            let _ = sender_clone.send(ComputorMessage::Tick);
        });

        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ComputorMessage::Start(t) => {
                    self.state.enabled = true;
                    self.state.ctype = t;
                }
                ComputorMessage::Stop => {
                    self.state.enabled = false;
                }
                ComputorMessage::Tick => {
                    self.handle_tick().await;

                    // Schedule next tick
                    let sender_clone = self.sender.clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(1)).await;
                        let _ = sender_clone.send(ComputorMessage::Tick);
                    });
                }
            }
        }
    }

    async fn handle_tick(&mut self) {
        if !self.state.enabled {
            return;
        }

        // Simulate quorum check
        if !fabric_sync_attest_gen::is_quorum_in_epoch().await {
            println!("🔴 cannot compute: out_of_sync");
            return;
        }

        // Run computation
        self.tick().await;
    }

    async fn tick(&self) {
        println!("computor running {:?}", chrono::Utc::now());

        let pk = AMACONFIG.trainer_pk();
        let pop = AMACONFIG.trainer_pop();

        let coins = Consensus::chain_balance(&pk).await;
        let epoch = Consensus::chain_epoch().await;
        let has_exec_coins = coins >= Coin::to_cents(100);

        if (self.state.ctype.as_deref() == Some("trainer") && !has_exec_coins)
            || self.state.ctype.is_none()
        {
            let sol = UPOW::compute_for(
                epoch,
                EntryGenesis::signer(),
                EntryGenesis::pop(),
                &pk,
                &crypto::strong_rand_bytes(96),
                100,
            )
            .await;

            if let Some(sol) = sol {
                println!("🔢 tensor matmul complete! broadcasting sol..");
                NodeGen::broadcast("sol", "trainers", &[sol]);
            }
        } else {
            let sol =
                UPOW::compute_for(epoch, &pk, &pop, &pk, &crypto::strong_rand_bytes(96), 100).await;

            if let Some(sol) = sol {
                let sk = AMACONFIG.trainer_sk;
                let packed_tx = TX::build(&sk, "Epoch", "submit_sol", &[sol.clone()]);
                let hash = TX::unpack(&packed_tx).hash;
                println!("🔢 tensor matmul complete! tx {:?}", base58::encode(hash));

                tx_pool::insert(packed_tx.clone());
                NodeGen::broadcast("txpool", "trainers", &[vec![packed_tx]]);
            }
        }
    }

    pub async fn set_emission_address(to_address: &str) {
        let sk = AMACONFIG.trainer_sk;
        let packed_tx = TX::build(
            &sk,
            "Epoch",
            "set_emission_address",
            &[to_address.to_string()],
        );
        tx_pool::insert(packed_tx.clone());
        NodeGen::broadcast("txpool", "trainers", &[vec![packed_tx]]);
    }
}
