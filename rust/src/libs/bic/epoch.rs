use crate::Coin;

pub struct Epoch;

impl Epoch {
    pub const EPOCH_EMISSION_BASE: i64 = Coin::to_flat(1_000_000);
    pub const EPOCH_EMISSION_FIXED: i64 = Coin::to_flat(100_000);

    pub const EPOCH_INTERVAL: i64 = 100_000;

    /// parameters for emission formula
    pub const A: f64 = 23_072_960_000.0;
    pub const C: f64 = 1110.573766;
    pub const START_EPOCH: i64 = 500;
}
