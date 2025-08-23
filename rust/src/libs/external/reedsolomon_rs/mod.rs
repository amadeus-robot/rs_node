use reed_solomon_simd::{ReedSolomonDecoder, ReedSolomonEncoder};
use std::sync::{Arc, Mutex};

/// A resource struct holding encoder and decoder
#[derive(Clone)]
pub struct ReedSolomonResource {
    pub encoder: Arc<Mutex<ReedSolomonEncoder>>,
    pub decoder: Arc<Mutex<ReedSolomonDecoder>>,
}

impl ReedSolomonResource {
    /// Create a new Reed-Solomon encoder/decoder resource
    pub fn new(data_shards: usize, recovery_shards: usize, shard_size: usize) -> Result<Self, &'static str> {
        let encoder = ReedSolomonEncoder::new(data_shards, recovery_shards, shard_size)
            .map_err(|_| "Failed to create encoder")?;
        let decoder = ReedSolomonDecoder::new(data_shards, recovery_shards, shard_size)
            .map_err(|_| "Failed to create decoder")?;

        Ok(Self {
            encoder: Arc::new(Mutex::new(encoder)),
            decoder: Arc::new(Mutex::new(decoder)),
        })
    }

    /// Encode data into shards
    pub fn encode(&self, data: &[u8], chunk_size: usize) -> Result<Vec<Vec<u8>>, &'static str> {
        let mut encoder = self.encoder.lock().map_err(|_| "Mutex poisoned")?;
        let chunk_count = (data.len() + chunk_size - 1) / chunk_size;

        let mut encoded_shards = Vec::with_capacity(chunk_count * 2);

        for chunk_start in (0..data.len()).step_by(chunk_size) {
            let chunk_end = (chunk_start + chunk_size).min(data.len());
            let chunk = &data[chunk_start..chunk_end];

            let mut buffer = vec![0u8; chunk_size];
            buffer[..chunk.len()].copy_from_slice(chunk);

            encoder.add_original_shard(&buffer).map_err(|_| "Failed to add original shard")?;
            encoded_shards.push(buffer);
        }

        let result = encoder.encode().map_err(|_| "Encoding failed")?;
        for recovered_shard in result.recovery_iter() {
            encoded_shards.push(recovered_shard.to_vec());
        }

        Ok(encoded_shards)
    }

    /// Decode shards back into original data
    pub fn decode(&self, shards: &[Vec<u8>], total_shards: usize, original_size: usize) -> Result<Vec<u8>, &'static str> {
        let mut decoder = self.decoder.lock().map_err(|_| "Mutex poisoned")?;
        let half = total_shards / 2;

        let mut combined = vec![0u8; original_size];

        for (i, shard) in shards.iter().enumerate() {
            if i < half {
                let offset = i * shard.len();
                let end = (offset + shard.len()).min(original_size);
                combined[offset..end].copy_from_slice(&shard[..(end - offset)]);
                decoder.add_original_shard(i, shard).map_err(|_| "Failed to add original shard")?;
            } else {
                decoder.add_recovery_shard(i - half, shard).map_err(|_| "Failed to add recovery shard")?;
            }
        }

        let result = decoder.decode().map_err(|_| "Decoding failed")?;
        for idx in 0..half {
            if let Some(shard_data) = result.restored_original(idx) {
                let offset = idx * shard_data.len();
                let end = (offset + shard_data.len()).min(original_size);
                combined[offset..end].copy_from_slice(&shard_data[..(end - offset)]);
            }
        }

        Ok(combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reed_solomon_encode_decode() {
        let data = b"Hello, Reed-Solomon! This is a test of encoding and decoding using Rust.".to_vec();
        let shard_size = 16;
        let data_shards = 4;
        let recovery_shards = 2;

        let rs = ReedSolomonResource::new(data_shards, recovery_shards, shard_size).unwrap();
        let encoded = rs.encode(&data, shard_size).unwrap();

        // simulate losing some shards
        let mut received_shards = encoded.clone();
        received_shards[1] = vec![0u8; shard_size]; // lost
        received_shards[4] = vec![0u8; shard_size]; // lost

        let decoded = rs.decode(&received_shards, data_shards + recovery_shards, data.len()).unwrap();
        assert_eq!(decoded, data);
    }
}
