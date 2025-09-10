use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task;
use tokio::time::sleep;

use crate::*;
/// Messages that control the Logger
#[derive(Debug)]
enum LoggerMsg {
    Start,
    Stop,
    Tick,
}

#[derive(Debug, Clone)]
pub struct LoggerState {
    pub enabled: bool,
}

pub struct LoggerGen {
    tx: UnboundedSender<LoggerMsg>,
}

impl LoggerGen {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let state = LoggerState { enabled: true };

        task::spawn(async move {
            LoggerGen::run(state, rx).await;
        });

        LoggerGen { tx }
    }

    pub fn start(&self) {
        let _ = self.tx.send(LoggerMsg::Start);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(LoggerMsg::Stop);
    }

    async fn run(mut state: LoggerState, mut rx: UnboundedReceiver<LoggerMsg>) {
        // first tick after 1s (like :erlang.send_after(1000))
        tokio::spawn({
            let tx = rx.sender().clone();
            async move {
                sleep(Duration::from_secs(1)).await;
                let _ = tx.send(LoggerMsg::Tick);
            }
        });

        while let Some(msg) = rx.recv().await {
            match msg {
                LoggerMsg::Start => {
                    state.enabled = true;
                }
                LoggerMsg::Stop => {
                    state.enabled = false;
                }
                LoggerMsg::Tick => {
                    if state.enabled {
                        // equivalent of try/catch -> tick(state) || state
                        match Self::tick(&state).await {
                            Ok(_) => {}
                            Err(e) => eprintln!("tick error: {:?}", e),
                        }
                    }
                    // re-schedule next tick after 6s
                    let tx = rx.sender().clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(6)).await;
                        let _ = tx.send(LoggerMsg::Tick);
                    });
                }
            }
        }
    }

    async fn tick(_state: &LoggerState) -> anyhow::Result<()> {
        // NOTE: placeholders for your modules
        let entry_rooted = Fabric::rooted_tip_entry().await;
        let rooted_height = entry_rooted.header_unpacked.height;

        let entry = Consensus::chain_tip_entry().await;
        let entry = Entry::unpack(entry).unwrap();
        let height = entry.header_unpacked.height;
        let slot = entry.header_unpacked.slot;
        let txpool_size = TxPool::size();
        let peer_cnt = NodePeers::online().len() + 1;

        let pk = Config::trainer_pk();
        let coins = Consensus::chain_balance(&pk).await;

        let trainers = Consensus::trainers_for_height(Entry::height(&entry) + 1).await;
        let is_trainer = if trainers.contains(&pk) {
            "💰"
        } else {
            "🪙"
        };

        let is_synced = FabricSyncAttestGen::is_quorum_synced_off_by_1().await;
        let highest_height = std::cmp::max(
            FabricSyncAttestGen::highest_temporal_height().unwrap_or(height),
            height,
        );

        let score = API::Epoch::score(&pk).score.unwrap_or(0);

        if !is_synced {
            println!(
                "⛓️  {} / {} R: {} | T: {} P: {} 🔴 NOT-SYNCED {}",
                height,
                highest_height,
                height - rooted_height,
                txpool_size,
                peer_cnt,
                Base58::encode(&pk)
            );
        } else {
            println!(
                "⛓️  {} / {} R: {} | T: {} P: {} S: {} | {} {} {}",
                height,
                highest_height,
                height - rooted_height,
                txpool_size,
                peer_cnt,
                score,
                Base58::encode(&pk),
                is_trainer,
                coins
            );
        }

        Ok(())
    }
}
