use blake3::{Hash, Hasher};
use rand::RngCore;
use rand::rngs::OsRng;

use crate::Sol;

pub struct UPOW {}

impl UPOW {
    pub fn compute_for(
        epoch: u64,
        trainer: &[u8],
        pop: &[u8],
        computor: &[u8],
        segment_vr: &[u8],
        mut itrs: usize,
    ) -> Option<Vec<u8>> {
        while itrs > 0 {
            let (hash, sol) = Self::branch_sol(epoch, trainer, pop, computor, segment_vr);
            if Sol::verify_hash(epoch, &hash) {
                return Some(sol);
            }
            itrs -= 1;
        }
        None
    }

    fn branch_sol(
        epoch: u64,
        trainer: &[u8],
        pop: &[u8],
        computor: &[u8],
        segment_vr: &[u8],
    ) -> (Vec<u8>, Vec<u8>) {
        if epoch >= 156 {
            UPOW2::tensormath(
                epoch,
                blake3::hash(segment_vr).as_bytes(),
                trainer,
                pop,
                computor,
            )
        } else if epoch >= 1 {
            UPOW1::tensormath(epoch, trainer, pop, computor, segment_vr)
        } else {
            UPOW0::tensormath(epoch, trainer, pop, computor)
        }
    }
}

pub struct UPOW0 {}

impl UPOW0 {
    pub fn tensormath(
        epoch: u64,
        trainer: &[u8],
        pop: &[u8],
        computor: &[u8],
    ) -> (Vec<u8>, Vec<u8>) {
        let mut nonce = vec![0u8; 32];
        OsRng.fill_bytes(&mut nonce);

        let mut sol_seed = Vec::new();
        sol_seed.extend(&epoch.to_le_bytes());
        sol_seed.extend(trainer);
        sol_seed.extend(pop);
        sol_seed.extend(computor);
        sol_seed.extend(&nonce);

        // pad to 256 bytes
        if sol_seed.len() > 256 {
            sol_seed.truncate(256);
        } else {
            sol_seed.resize(256, 0u8);
        }

        let hash = Self::calculate(&sol_seed);
        (hash, sol_seed)
    }

    fn calculate(sol_seed: &[u8]) -> Vec<u8> {
        // create Blake3 hasher and seed
        let mut hasher = Hasher::new();
        hasher.update(sol_seed);

        // build tensor: map idx -> 1024 bytes (Vec<u8>) for idx in 0..1023
        let mut tensor: Vec<Vec<u8>> = Vec::with_capacity(1024);
        for _idx in 0..1024 {
            // finalize_xof 1024 bytes
            let mut xof = hasher.finalize_xof();
            let mut out = vec![0u8; 1024];
            xof.fill(&mut out);
            tensor.push(out);

            // update hasher with non-XOF finalize (32 bytes)
            let h = hasher.finalize();
            hasher.update(h.as_bytes());
        }

        // produce random_walk_bin: 1024*8*2 bytes (same as Elixir)
        let rnd_len = 1024 * 8 * 2;
        let mut xof = hasher.finalize_xof();
        let mut random_walk_bin = vec![0u8; rnd_len];
        xof.fill(&mut random_walk_bin);

        Self::walk_mul(&random_walk_bin, &mut tensor)
    }

    fn walk_mul(random_walk: &[u8], tensor: &mut [Vec<u8>]) -> Vec<u8> {
        // random_walk is sequence of little 16-bit indices
        let mut idx = 0usize;
        while idx + 1 < random_walk.len() {
            let low = random_walk[idx] as u16;
            let high = random_walk[idx + 1] as u16;
            let index = ((high << 8) | low) as usize;
            let index = index % 1024;

            // take row tensor[index] and produce new_row
            let row = &tensor[index];
            // new_row: for each byte in row, take byte, square modulo 256 (wrapping)
            let mut new_row = Vec::with_capacity(row.len());
            for &b in row.iter().take(1024) {
                let sq = b.wrapping_mul(b);
                new_row.push(sq);
            }

            tensor[index] = new_row;
            idx += 2;
        }

        // when random_walk exhausted, compress tensor by hashing concatenation
        let mut final_hasher = Hasher::new();
        for i in 0..1024 {
            final_hasher.update(&tensor[i]);
        }
        let final_hash = final_hasher.finalize();
        final_hash.as_bytes().to_vec()
    }
}
pub struct UPOW1 {}

impl UPOW1 {
    pub fn tensormath(
        epoch: u64,
        trainer: &[u8],
        pop: &[u8],
        computor: &[u8],
        segment_vr: &[u8],
    ) -> (Vec<u8>, Vec<u8>) {
        let mut nonce = vec![0u8; 16];
        OsRng.fill_bytes(&mut nonce);

        let mut sol_seed = Vec::new();
        sol_seed.extend(&epoch.to_le_bytes());
        sol_seed.extend(trainer);
        sol_seed.extend(pop);
        sol_seed.extend(computor);
        sol_seed.extend(segment_vr);
        sol_seed.extend(&nonce);

        // pad to 320 bytes
        if sol_seed.len() > 320 {
            sol_seed.truncate(320);
        } else {
            sol_seed.resize(320, 0u8);
        }

        let hash = Self::calculate(&sol_seed);
        (hash, sol_seed)
    }

    fn calculate(sol_seed: &[u8]) -> Vec<u8> {
        let mut hasher = Hasher::new();
        hasher.update(sol_seed);

        // tensor: 256 rows each 256 bytes
        let mut tensor: Vec<Vec<u8>> = Vec::with_capacity(256);
        for _ in 0..256 {
            let mut xof = hasher.finalize_xof();
            let mut out = vec![0u8; 256];
            xof.fill(&mut out);
            tensor.push(out);

            // update with finalize 32 bytes
            let h = hasher.finalize();
            hasher.update(h.as_bytes());
        }

        // random_walk_bin: 512*8*2 bytes
        let rnd_len = 512 * 8 * 2;
        let mut xof = hasher.finalize_xof();
        let mut random_walk_bin = vec![0u8; rnd_len];
        xof.fill(&mut random_walk_bin);

        Self::walk_mul(&random_walk_bin, &mut tensor)
    }

    fn walk_mul(random_walk: &[u8], tensor: &mut [Vec<u8>]) -> Vec<u8> {
        // indices are 16-bit little
        let mut idx = 0usize;
        while idx + 1 < random_walk.len() {
            let low = random_walk[idx] as u16;
            let high = random_walk[idx + 1] as u16;
            let index = ((high << 8) | low) as usize;
            let index = index % 256;

            let row = &tensor[index];
            let mut new_row = Vec::with_capacity(row.len());
            for &b in row.iter().take(256) {
                let sq = b.wrapping_mul(b);
                new_row.push(sq);
            }

            tensor[index] = new_row;
            idx += 2;
        }

        let mut final_hasher = Hasher::new();
        for i in 0..256 {
            final_hasher.update(&tensor[i]);
        }
        final_hasher.finalize().as_bytes().to_vec()
    }
}

pub struct UPOW2 {}

impl UPOW2 {
    pub fn tensormath(
        epoch: u64,
        segment_vr_hash: &[u8],
        trainer: &[u8],
        pop: &[u8],
        computor: &[u8],
    ) -> (Vec<u8>, Vec<u8>) {
        let mut nonce = vec![0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce);

        let mut sol_seed = Vec::new();
        sol_seed.extend(&epoch.to_le_bytes());
        sol_seed.extend(segment_vr_hash);
        sol_seed.extend(trainer);
        sol_seed.extend(pop);
        sol_seed.extend(computor);
        sol_seed.extend(&nonce);

        let tensor_c = Self::calculate_matmul(&sol_seed);
        let mut sol = Vec::with_capacity(sol_seed.len() + tensor_c.len());
        sol.extend(&sol_seed);
        sol.extend(&tensor_c);
        let hash = blake3::hash(&sol);
        (hash.as_bytes().to_vec(), sol)
    }

    fn calculate_matmul(sol_seed: &[u8]) -> Vec<u8> {
        // require sol_seed length == 240 (as in Elixir guard)
        if sol_seed.len() != 240 {
            panic!("sol_seed must be exactly 240 bytes for UPOW2.calculate_matmul");
        }

        let mut hasher = Hasher::new();
        hasher.update(sol_seed);

        // sizes requested:
        // matrix_a: 16 * 50240 bytes
        // matrix_b: 50240 * 16 bytes
        // matrix_b2: 16 * 64 bytes (not used)
        let a_len = 16usize * 50_240usize;
        let b_len = 50_240usize * 16usize;
        let b2_len = 16usize * 64usize;
        let total = a_len + b_len + b2_len;

        let mut xof = hasher.finalize_xof();
        let mut big = vec![0u8; total];
        xof.fill(&mut big);

        let matrix_a = &big[0..a_len];
        let matrix_b = &big[a_len..(a_len + b_len)];
        // let matrix_b2 = &big[(a_len + b_len)..]; // unused in current logic

        // multiply and convert to bytes
        let c_bytes = MatrixMul::multiply_to_bytes(matrix_a, matrix_b);
        c_bytes
    }
}

pub mod MatrixMul {
    /// Mirror of Elixir MatrixMul
    /// rows = 16, cols = 16, k_dim = 50_240
    pub fn multiply_to_bytes(a_bin: &[u8], b_bin: &[u8]) -> Vec<u8> {
        const ROWS: usize = 16;
        const COLS: usize = 16;
        const K_DIM: usize = 50_240;

        assert_eq!(a_bin.len(), ROWS * K_DIM);
        assert_eq!(b_bin.len(), K_DIM * COLS);

        // produce i32 little-endian per (i,j)
        let mut out = Vec::with_capacity(ROWS * COLS * 4);

        for i in 0..ROWS {
            for j in 0..COLS {
                // compute dot product sum over k
                let mut sum: i64 = 0;
                for k in 0..K_DIM {
                    let a_val = a_bin[i * K_DIM + k] as i64; // unsigned byte
                    let b_val = b_bin[k * COLS + j] as i8 as i64; // signed byte
                    sum += a_val * b_val;
                }
                // cast to i32 (wrapping)
                let s32 = sum as i32;
                out.extend(&s32.to_le_bytes());
            }
        }

        out
    }
}
