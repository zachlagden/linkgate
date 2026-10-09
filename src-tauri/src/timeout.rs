use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub const MIN_SECONDS: u32 = 3;
pub const MAX_SECONDS: u32 = 60;
pub const DEFAULT_SECONDS: u32 = 10;

pub fn clamp(seconds: u32) -> u32 {
    seconds.clamp(MIN_SECONDS, MAX_SECONDS)
}

fn from_value(value: &Value) -> u32 {
    match value.as_f64() {
        Some(number) if number.is_finite() => clamp(number.round().max(0.0) as u32),
        _ => DEFAULT_SECONDS,
    }
}

pub fn default_seconds() -> u32 {
    DEFAULT_SECONDS
}

pub fn deserialize_seconds<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    Value::deserialize(deserializer).map(|value| from_value(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_values_in_range() {
        assert_eq!(from_value(&json!(3)), 3);
        assert_eq!(from_value(&json!(25)), 25);
        assert_eq!(from_value(&json!(60)), 60);
    }

    #[test]
    fn raises_values_below_the_minimum() {
        assert_eq!(from_value(&json!(0)), MIN_SECONDS);
        assert_eq!(from_value(&json!(1)), MIN_SECONDS);
        assert_eq!(from_value(&json!(-5)), MIN_SECONDS);
    }

    #[test]
    fn lowers_values_above_the_maximum() {
        assert_eq!(from_value(&json!(61)), MAX_SECONDS);
        assert_eq!(from_value(&json!(100000)), MAX_SECONDS);
        assert_eq!(from_value(&json!(1e30)), MAX_SECONDS);
    }

    #[test]
    fn rounds_fractions() {
        assert_eq!(from_value(&json!(7.6)), 8);
    }

    #[test]
    fn falls_back_to_the_default_for_other_types() {
        assert_eq!(from_value(&json!("fast")), DEFAULT_SECONDS);
        assert_eq!(from_value(&json!(null)), DEFAULT_SECONDS);
        assert_eq!(from_value(&json!(true)), DEFAULT_SECONDS);
        assert_eq!(from_value(&json!([5])), DEFAULT_SECONDS);
    }

    #[test]
    fn clamp_limits_both_ends() {
        assert_eq!(clamp(0), MIN_SECONDS);
        assert_eq!(clamp(500), MAX_SECONDS);
        assert_eq!(clamp(12), 12);
    }
}
