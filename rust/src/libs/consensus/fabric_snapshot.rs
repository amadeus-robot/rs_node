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

    // pub fn download_latest(height: u64) {
    //     let padded = format!("{:012}", height);
    //     println!("quick-syncing chain snapshot height {}..", height);
    //     let url = format!("https://snapshots.amadeus.bot/{}.zip", padded);

    //     let cwd_dir = Path::new("/tmp/updates_tmp/");
    //     fs::create_dir_all(cwd_dir).unwrap();

    //     let zip_path = cwd_dir.join(format!("{}.zip", padded));
    //     let resp = reqwest_blocking::get(&url).unwrap().bytes().unwrap();
    //     fs::write(&zip_path, &resp).unwrap();
    //     println!("quick-sync download complete. Extracting..");

    //     let file = File::open(&zip_path).unwrap();
    //     let mut archive = ZipArchive::new(file).unwrap();
    //     archive.extract("/var/work/fabric/").unwrap();

    //     fs::remove_file(zip_path).unwrap();
    //     println!("quick-sync done");
    // }

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