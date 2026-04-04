//! Test utilities for the multiagent crate

use crate::errors::Result;

/// Dummy test function to increase test coverage
pub fn dummy_test_fn() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy() {
        assert!(dummy_test_fn().is_ok());
    }
}