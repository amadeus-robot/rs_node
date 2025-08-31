use crate::*;

pub struct Coin;

impl Coin {
    pub const DECIMALS: u32 = 9;
    pub const BURN_ADDRESS: &'static str = "000000000000000000000000000000000000000000000000";

    pub const fn to_flat(coins: i64) -> i64 {
        coins * 1_000_000_000
    }

    pub fn to_cents(coins: i128) -> i128 {
        coins * 10_000_000
    }

    pub fn to_tenthousandth(coins: i128) -> i128 {
        coins * 100_000
    }

    pub fn from_flat(coins: i128) -> f64 {
        (coins as f64) / 1_000_000_000.0
    }

    pub fn burn_address() -> &'static str {
        Self::BURN_ADDRESS
    }

    pub fn burn_balance(symbol: &str) -> i64 {
        Self::balance(Self::BURN_ADDRESS, symbol)
    }

    pub fn balance(pubkey: &str, symbol: &str) -> i64 {
        let key = format!("bic:coin:balance:{}:{}", pubkey, symbol);
        let raw_value: Option<Vec<u8>> = ConsensusKV::kv_get(&key.as_bytes());
        if let Some(value) = raw_value {
            i64::from_be_bytes(value[..8].try_into().unwrap())
        } else {
            0
        }
    }
}

// Override ConsensusKV for testing
mod coin_tests {
    use super::*;

    #[test]
    fn test_to_flat() {
        assert_eq!(Coin::to_flat(1), 1_000_000_000);
        assert_eq!(Coin::to_flat(123), 123_000_000_000);
    }

    #[test]
    fn test_to_cents() {
        assert_eq!(Coin::to_cents(1), 10_000_000);
        assert_eq!(Coin::to_cents(50), 500_000_000);
    }

    #[test]
    fn test_to_tenthousandth() {
        assert_eq!(Coin::to_tenthousandth(1), 100_000);
        assert_eq!(Coin::to_tenthousandth(20), 2_000_000);
    }

    #[test]
    fn test_from_flat() {
        let val = Coin::from_flat(1_000_000_000);
        assert!((val - 1.0).abs() < 1e-9);

        let val2 = Coin::from_flat(500_000_000);
        assert!((val2 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_burn_address() {
        assert_eq!(
            Coin::burn_address(),
            "000000000000000000000000000000000000000000000000"
        );
    }

    //  Fabric not initialized
    #[test]
    fn test_balance_zero() {
        let pubkey = "pubkey1";
        let symbol = "SYM";
        let balance = Coin::balance(pubkey, symbol);
        assert_eq!(balance, 0);
    }

    #[test]
    fn test_balance_set() {
        let pubkey = "pubkey1";
        let symbol = "SYM";
        let value: i64 = 12345;
        // ConsensusKV::kv_set(
        //     format!("bic:coin:balance:{}:{}", pubkey, symbol).as_bytes(),
        //     &value.to_be_bytes(),
        // );

        let balance = Coin::balance(pubkey, symbol);
        assert_eq!(balance, value);
    }

    #[test]
    fn test_burn_balance_set() {
        let symbol = "SYM";
        let value: i64 = 999;
        // ConsensusKV::kv_set(
        //     format!("bic:coin:balance:{}:{}", Coin::BURN_ADDRESS, symbol).as_bytes(),
        //     &value.to_be_bytes(),
        // );

        let balance = Coin::burn_balance(symbol);
        assert_eq!(balance, value);
    }
}
