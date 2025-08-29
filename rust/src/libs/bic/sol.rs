pub struct Sol;

impl Sol {
    pub const PREAMBLE_SIZE: usize = 240;
    pub const MATRIX_SIZE: usize = 1024;
    pub const SOL_SIZE: usize = Self::PREAMBLE_SIZE + Self::MATRIX_SIZE;

    /// Return total size of a solution
    pub fn size() -> usize {
        Self::SOL_SIZE
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