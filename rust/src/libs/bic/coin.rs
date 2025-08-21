pub struct Coin;

impl Coin {
    pub const DECIMALS: u32 = 9;
    pub const BURN_ADDRESS: &'static str = "000000000000000000000000000000000000000000000000";

    pub fn to_flat(coins: i128) -> i128 {
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
        Coin::BURN_ADDRESS
    }

    pub fn burn_balance(symbol: &str) -> i64 {
        Coin::balance(Self::BURN_ADDRESS, symbol)
    }

    pub fn balance(pubkey: &str, symbol: &str) -> i64 {
        let key = format!("bic:coin:balance:{}:{}", pubkey, symbol);
        kv_get_int(&key).unwrap_or(0)
    }
}
