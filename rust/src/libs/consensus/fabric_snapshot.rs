use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};
use zip::ZipArchive;

use crate::*;

pub struct FabricSnapshot {}

impl FabricSnapshot {
    pub fn prune() {
        let end_hash = Fabric::pruned_hash();
        let start_hash = Fabric::rooted_tip();
        let guard = FABRIC_DB.read().unwrap();
        let fabric = guard.as_ref().unwrap();

        // Self::walk(end_hash, start_hash, fabric);
    }

    // fn walk(end_hash: Vec<u8>, start_hash: Vec<u8>, fabric: &Fabric) {
    //     let entry = Fabric::entry_by_hash(&start_hash);
    //     let height = entry.height;
    //     println!("walk height={}", height);

    //     let mut entries = Fabric::entries_by_height(height);
    //     entries.retain(|e| e.hash != entry.hash);

    //     // delete from RocksDB
    //     let wo = WriteOptions::default();
    //     fabric.db.delete_cf(&fabric.cf.my_attestation_for_entry, &entry.hash).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.muts_rev, &entry.hash).unwrap();

    //     let map = Fabric::consensuses_by_entryhash(&entry.hash);
    //     if map.len() != 1 {
    //         panic!("Consensus mismatch at height {} map={:?}", height, map);
    //     }

    //     for e in entries {
    //         println!("delete height={} hash={:?}", height, e.hash);
    //         Self::delete_entry_and_metadata(&e, fabric);
    //     }

    //     if entry.hash == end_hash {
    //         return;
    //     } else if let Some(prev_hash) = entry.prev_hash {
    //         Self::walk(end_hash, prev_hash, fabric);
    //     }
    // }

    // fn delete_entry_and_metadata(entry: &Entry, fabric: &Fabric) {
    //     let height = entry.height;
    //     let hash = &entry.hash;
    //     let wo = WriteOptions::default();

    //     fabric.db.delete_cf(&fabric.cf.default, hash).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.entry_by_height, format!("{}:{:?}", height, hash)).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.entry_by_slot, format!("{}:{:?}", height, hash)).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.consensus_by_entryhash, hash).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.my_attestation_for_entry, hash).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.muts, hash).unwrap();
    //     fabric.db.delete_cf(&fabric.cf.muts_rev, hash).unwrap();
    // }

    // pub fn backstep_temporal(list: Vec<Vec<u8>>) {
    //     let guard = FABRIC_DB.read().unwrap();
    //     let fabric = guard.as_ref().unwrap();

    //     for hash in list.into_iter().rev() {
    //         let entry = Fabric::entry_by_hash(&hash);
    //         let in_chain = Consensus::is_in_chain(&hash);
    //         if in_chain {
    //             assert!(Consensus::chain_rewind(&hash));
    //         }
    //         if let Some(e) = entry {
    //             Self::delete_entry_and_metadata(&e, fabric);
    //         }
    //     }
    // }
    pub async fn download_latest() -> Result<(), Box<dyn std::error::Error>> {
        let height: u64 = AMACONFIG.snapshot_height;
        let padded = format!("{:012}", height);
        println!(
            "Quick-syncing chain snapshot height {}.. this can take a while",
            height
        );

        // let url = format!("https://snapshots.amadeus.bot/{}.zip", padded);
        // println!("{}", url);

        let work_folder = AMACONFIG.work_folder.clone();
        let cwd_dir = Path::new(&work_folder).join("updates_tmp");
        std::fs::create_dir_all(&cwd_dir)?;

        let zip_path = cwd_dir.join(format!("{}.zip", padded));

        // --- DOWNLOAD ---
        let client = Client::new();
        // let resp = client.get(&url).send().await?;
        // let total_size = resp
        //     .content_length()
        //     .ok_or("Failed to get content length")?;

    //     let pb = ProgressBar::new(total_size);
    //     pb.set_style(ProgressStyle::with_template(
    //     "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)"
    // )?);

    //     let mut file = tokio::fs::File::create(&zip_path).await?;

    //     let mut stream = resp.bytes_stream(); // this is a Stream of Result<Bytes, Error>
    //     while let Some(item) = stream.next().await {
    //         let chunk = item?; // chunk: bytes::Bytes
    //         tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
    //         pb.inc(chunk.len() as u64); // increment progress bar
    //     }
    //     pb.finish_with_message("Download complete");

        // --- EXTRACTION ---
        println!("Extracting snapshot...");
        let file = File::open(&zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        // total uncompressed size
        let total_uncompressed: u64 = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().size())
            .sum();

        let extract_pb = ProgressBar::new(total_uncompressed);
        extract_pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.magenta/blue}] {bytes}/{total_bytes} ({percent}%)"
    )?);

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = Path::new("/var/work/fabric/").join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(&p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                let mut buffer = [0u8; 8 * 1024];
                loop {
                    let n = file.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    outfile.write_all(&buffer[..n])?;
                    extract_pb.inc(n as u64);
                }
            }
        }

        extract_pb.finish_with_message("Extraction complete");
        tokio::fs::remove_file(zip_path).await?;
        println!("Quick-sync done");
        Ok(())
    }
    // pub fn snapshot_tmp() -> u64 {
    //     let height = Fabric::rooted_tip_height();
    //     let padded = format!("{:012}", height);
    //     let dir = format!("/tmp/{}/db/", padded);
    //     fs::create_dir_all(&dir).unwrap();
    //     // simulate checkpoint
    //     println!("checkpoint created at {}", dir);
    //     height
    // }

    // pub fn upload_latest() {
    //     // in real use: aws s3 cp commands would be invoked via std::process::Command
    //     println!("simulate upload snapshot to cloudflare s3");
    // }
}
