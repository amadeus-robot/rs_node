use {
    crate::*,
    std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    },
    tokio::{task, time::sleep},
};

pub mod consensus;
pub mod http;
pub mod misc;
pub mod node;
pub use consensus::*;
pub use http::*;
pub use misc::*;
pub use node::*;

pub struct AmaApp {
    pub node_inited: Arc<AtomicBool>,
}

impl AmaApp {
    pub fn new() -> Self {
        Self {
            node_inited: Arc::new(AtomicBool::new(false)),
        }
    }

    async fn autoupdate_task() {
        // Placeholder for auto-update logic
        println!("Running auto-update...");
    }

    async fn spawn_child_tasks(&self) {
        // Examples of child tasks
        task::spawn(async {
            // ComputorGen::start_link();
            println!("ComputorGen started");
        });

        task::spawn(async {
            // LoggerGen::start_link();
            println!("LoggerGen started");
        });

        task::spawn(async {
            // FabricGen::start_link();
            println!("FabricGen started");
        });

        // You can spawn more tasks dynamically
    }

    pub async fn start(&self) {
        // 1. Startup delay
        sleep(Duration::from_millis(300)).await;

        if AMACONFIG.autoupdate {
            println!("🟢 Auto-update enabled");
            // spawn auto-update task
            task::spawn(Self::autoupdate_task());
        }

        let _ = Fabric::init();

        let _ = TXPool::init();

        if !AMACONFIG.offline {
            let rooted_tip_raw_height = Fabric::rooted_tip_height();

            if let Some(rooted_tip_height) = rooted_tip_raw_height {
                if rooted_tip_height < AMACONFIG.snapshot_height {
                    let _ = Fabric::init();

                    Fabric::close();
                }
            };
            // Check snapshot logic
            Fabric::close();
            let _ = FabricSnapshot::download_latest().await;
        } else {
            // Offline init placeholder
            //~ Consensus.apply_entry(...);          //  WORKING PART
        }

        // Spawn supervised tasks
        self.spawn_child_tasks().await;
    }

    pub fn wait_node_inited(&self, timeout_ms: Option<u64>) -> bool {
        let timeout = timeout_ms.unwrap_or(10 * 60_000); // default 10 min
        let start = Instant::now();

        while !self.node_inited.load(Ordering::SeqCst) {
            if start.elapsed() > Duration::from_millis(timeout) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(333));
        }

        true
    }
}
