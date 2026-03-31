use crate::types::Usage;

/// Accumulate usage from API response into a running total.
pub fn accumulate_usage(total: &mut Usage, response: &Usage) {
    total.input_tokens += response.input_tokens;
    total.output_tokens += response.output_tokens;
    total.cache_read_input_tokens += response.cache_read_input_tokens;
    total.cache_creation_input_tokens += response.cache_creation_input_tokens;
}

/// Calculate total tokens (input + output).
pub fn total_tokens(usage: &Usage) -> u64 {
    usage.input_tokens + usage.output_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulate_usage_adds_values() {
        let mut total = Usage::default();
        let response = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 20,
            cache_creation_input_tokens: 10,
        };
        accumulate_usage(&mut total, &response);
        assert_eq!(total.input_tokens, 100);
        assert_eq!(total.output_tokens, 50);
        assert_eq!(total.cache_read_input_tokens, 20);
        assert_eq!(total.cache_creation_input_tokens, 10);
    }

    #[test]
    fn accumulate_usage_multiple_times() {
        let mut total = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 20,
            cache_creation_input_tokens: 10,
        };
        let response = Usage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_input_tokens: 30,
            cache_creation_input_tokens: 5,
        };
        accumulate_usage(&mut total, &response);
        assert_eq!(total.input_tokens, 300);
        assert_eq!(total.output_tokens, 150);
        assert_eq!(total.cache_read_input_tokens, 50);
        assert_eq!(total.cache_creation_input_tokens, 15);
    }

    #[test]
    fn accumulate_usage_with_zero_response() {
        let mut total = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        };
        let response = Usage::default();
        accumulate_usage(&mut total, &response);
        assert_eq!(total.input_tokens, 100);
        assert_eq!(total.output_tokens, 50);
    }

    #[test]
    fn total_tokens_basic() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 20,
            cache_creation_input_tokens: 10,
        };
        assert_eq!(total_tokens(&usage), 150);
    }

    #[test]
    fn total_tokens_zero() {
        let usage = Usage::default();
        assert_eq!(total_tokens(&usage), 0);
    }

    #[test]
    fn total_tokens_only_input() {
        let usage = Usage {
            input_tokens: 500,
            output_tokens: 0,
            ..Default::default()
        };
        assert_eq!(total_tokens(&usage), 500);
    }

    #[test]
    fn total_tokens_only_output() {
        let usage = Usage {
            input_tokens: 0,
            output_tokens: 300,
            ..Default::default()
        };
        assert_eq!(total_tokens(&usage), 300);
    }
}
