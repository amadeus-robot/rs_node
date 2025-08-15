use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub work_folder: PathBuf,
    pub offline: bool,
    pub http_ipv4: String,
    pub http_port: u16,
    pub udp_ipv4: String,
    pub udp_port: u16,
    pub seed_nodes: Vec<String>,
    pub other_nodes: Vec<String>,
    pub trust_factor: f64,
    pub archival_node: bool,
    pub auto_update: bool,
    pub computor_type: Option<ComputorType>,
    pub snapshot_height: u64,
    pub entry_size: usize,
    pub tx_size: usize,
    pub attestation_size: usize,
    pub quorum: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputorType {
    Default,
    Trainer,
}


impl Config {
    pub fn load() -> Result<Self> {
        // Load environment variables
        dotenv::dotenv().ok();
        
        let work_folder = env::var("WORKFOLDER")
            .unwrap_or_else(|_| "~/.cache/amadeusd/".to_string())
            .into();
        
        let offline = env::var("OFFLINE").is_ok();
        let http_ipv4 = env::var("HTTP_IPV4").unwrap_or_else(|_| "0.0.0.0".to_string());
        let http_port = env::var("HTTP_PORT")
            .unwrap_or_else(|_| "80".to_string())
            .parse()
            .unwrap_or(80);
        
        let udp_ipv4 = env::var("UDP_IPV4").unwrap_or_else(|_| "0.0.0.0".to_string());
        let udp_port = 36969; // Fixed port as per original
        
        let seed_nodes = vec![
            "104.218.45.23".to_string(),
            "72.9.144.110".to_string(),
        ];
        
        let other_nodes = env::var("OTHERNODES")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        
        let trust_factor = env::var("TRUSTFACTOR")
            .unwrap_or_else(|_| "0.8".to_string())
            .parse()
            .unwrap_or(0.8);
        
        let archival_node = env::var("ARCHIVALNODE")
            .map(|s| matches!(s.as_str(), "true" | "y" | "yes"))
            .unwrap_or(false);
        
        let auto_update = env::var("AUTOUPDATE")
            .map(|s| matches!(s.as_str(), "true" | "y" | "yes"))
            .unwrap_or(false);
        
        let computor_type = env::var("COMPUTOR").map(|s| match s.as_str() {
            "trainer" => ComputorType::Trainer,
            _ => ComputorType::Default,
        });
        
        let snapshot_height = env::var("SNAPSHOT_HEIGHT")
            .unwrap_or_else(|_| "24875547".to_string())
            .parse()
            .unwrap_or(24875547);
        
        Ok(Config {
            version: env!("CARGO_PKG_VERSION").to_string(),
            work_folder,
            offline,
            http_ipv4,
            http_port,
            udp_ipv4,
            udp_port,
            seed_nodes,
            other_nodes,
            trust_factor,
            archival_node,
            auto_update,
            computor_type,
            snapshot_height,
            entry_size: 524288,
            tx_size: 393216,
            attestation_size: 512,
            quorum: 3,
        })
    }
    
    pub fn get_trainer_keys(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let sk_path = self.work_folder.join("sk");
        
        if !sk_path.exists() {
            // Generate new key pair
            let sk = crypto::generate_random_bytes(64)?;
            let pk = crypto::get_public_key(&sk)?;
            
            // Save private key
            std::fs::create_dir_all(&self.work_folder)?;
            std::fs::write(&sk_path, base64::encode(&sk))?;
            
            Ok((sk, pk))
        } else {
            // Load existing private key
            let sk_encoded = std::fs::read_to_string(&sk_path)?;
            let sk = base64::decode(sk_encoded.trim())?;
            let pk = crypto::get_public_key(&sk)?;
            
            Ok((sk, pk))
        }
    }
}
