use std::collections::HashMap;

pub struct Sol {
    pub epoch : u64,
    pub pk : Vec<u8>,
    pub pop : Vec<u8>,
    pub computor : u64,
    pub segment_vr_hash : Vec<u8>,
    pub nonce : u64,
    pub tensor_c : u64,
}

impl Sol {
    pub const PREAMBLE_SIZE: usize = 240;
    pub const MATRIX_SIZE: usize = 1024;
    pub const SOL_SIZE: usize = Self::PREAMBLE_SIZE + Self::MATRIX_SIZE;

    /// Return total size of a solution
    pub fn size() -> usize {
        Self::SOL_SIZE
    }

    pub fn unpack(sol: &[u8]) -> HashMap<&'static str, Vec<u8>> {
        let epoch = u32::from_le_bytes(sol[0..4].try_into().unwrap());
        let mut map = HashMap::new();

        if epoch >= 156 {
            let segment_vr_hash = sol[4..36].to_vec();
            let sol_pk = sol[36..84].to_vec();
            let pop = sol[84..180].to_vec();
            let computor_pk = sol[180..228].to_vec();
            let nonce = sol[228..240].to_vec();
            let tensor_c = sol[240..1248].to_vec();

            map.insert("epoch", epoch.to_le_bytes().to_vec());
            map.insert("pk", sol_pk);
            map.insert("pop", pop);
            map.insert("computor", computor_pk);
            map.insert("segment_vr_hash", segment_vr_hash);
            map.insert("nonce", nonce);
            map.insert("tensor_c", tensor_c);
        } else if epoch >= 1 {
            let sol_pk = sol[4..52].to_vec();
            let pop = sol[52..148].to_vec();
            let computor_pk = sol[148..196].to_vec();
            let segment_vr = sol[196..292].to_vec();

            map.insert("epoch", epoch.to_le_bytes().to_vec());
            map.insert("pk", sol_pk);
            map.insert("pop", pop);
            map.insert("computor", computor_pk);
            map.insert("segment_vr", segment_vr);
        } else {
            let sol_pk = sol[4..52].to_vec();
            let pop = sol[52..148].to_vec();
            let computor_pk = sol[148..196].to_vec();

            map.insert("epoch", epoch.to_le_bytes().to_vec());
            map.insert("pk", sol_pk);
            map.insert("pop", pop);
            map.insert("computor", computor_pk);
        }
        map
    }

    pub fn verify_hash(epoch: u64, hash: &[u8]) -> bool {
        if epoch >= 244 {
            hash[0] == 0 && hash[1] == 0 && hash[2] == 0
        } else if epoch >= 156 {
            hash[0] == 0 && hash[1] == 0
        } else if epoch >= 1 {
            hash[0] == 0 && hash[1] == 0
        } else {
            hash[0] == 0
        }
    }

    pub fn verify(sol: &[u8], opts: Option<HashMap<&str, Vec<u8>>>) -> Result<bool, String> {
        let epoch = u32::from_le_bytes(sol[0..4].try_into().unwrap());

        if epoch >= 260 {
            if sol.len() != Self::SOL_SIZE {
                return Err("invalid_sol_seed_size".into());
            }
            let hash = opts
                .as_ref()
                .and_then(|o| o.get("hash"))
                .cloned()
                .unwrap_or_else(|| blake3::hash(sol).as_bytes().to_vec());

            let vr_b3 = opts
                .as_ref()
                .and_then(|o| o.get("vr_b3"))
                .cloned()
                .unwrap_or_else(|| rand::random::<[u8; 32]>().to_vec());

            Ok(Self::verify_hash(epoch, &hash) && Blake3::freivalds_e260(sol, &vr_b3))
        } else if epoch >= 156 {
            if sol.len() != Self::SOL_SIZE {
                return Err("invalid_sol_seed_size".into());
            }
            let hash = opts
                .as_ref()
                .and_then(|o| o.get("hash"))
                .cloned()
                .unwrap_or_else(|| Blake3::hash(sol).as_bytes().to_vec());

            Ok(Self::verify_hash(epoch, &hash) && Blake3::freivalds(sol))
        } else if epoch >= 1 {
            if sol.len() != 320 {
                return Err("invalid_sol_seed_size".into());
            }
            Ok(Self::verify_cache("UPOW1", sol))
        } else {
            if sol.len() != 256 {
                return Err("invalid_sol_seed_size".into());
            }
            Ok(Self::verify_cache("UPOW0", sol))
        }
    }

    fn verify_cache(module: &str, sol: &[u8]) -> bool {
        // TODO: Replace with your ETS-like cache.
        // Pseudocode:
        // if cache says "valid" -> remove & return true
        // else -> recalc with UPOW0/UPOW1::calculate
        let epoch = u32::from_le_bytes(sol[0..4].try_into().unwrap());
        let hash = match module {
            "UPOW0" => upow0_calculate(sol),
            "UPOW1" => upow1_calculate(sol),
            _ => return false,
        };
        Self::verify_hash(epoch, &hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sol_size() {
        assert_eq!(Sol::size(), 1264); // 240 + 1024
    }
}
