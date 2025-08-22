pub struct ConsensusWeight;

impl ConsensusWeight {
    pub fn count(_pk: &[u8]) -> u32 {
        1
    }
}

#[cfg(test)]
mod consensus_weight_tests {
    use super::*;

    #[test]
    fn test_count_returns_one() {
        let pk = b"some_public_key";
        let weight = ConsensusWeight::count(pk);
        assert_eq!(weight, 1);
    }

    #[test]
    fn test_count_with_different_keys() {
        let keys = vec![
            b"pk1".as_ref(),
            b"pk2".as_ref(),
            b"another_key".as_ref(),
        ];

        for pk in keys {
            assert_eq!(ConsensusWeight::count(pk), 1);
        }
    }
}
