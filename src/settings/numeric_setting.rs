use crate::settings::setting_item::SettingItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(bound = "")]
pub struct NumericSetting {
    label: String,
    value: i64,
    step: i64,
    unit: String,
}

impl NumericSetting {
    pub fn new(label: &'static str, default: i64, step: i64, unit: &'static str) -> Self {
        Self {
            label: label.into(),
            value: default,
            step,
            unit: unit.into(),
        }
    }
}

#[typetag::serde]
impl SettingItem for NumericSetting {
    fn label(&self) -> &str {
        &self.label
    }

    fn get_value(&self) -> i64 {
        self.value
    }

    fn display_value(&self) -> String {
        format!("{}: {} {}", self.label, self.value, self.unit)
    }

    fn increment(&mut self) {
        self.value = self.value.saturating_add(self.step);
    }

    fn decrement(&mut self) {
        self.value = self.value.saturating_sub(self.step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_initial_value() {
        let s = NumericSetting::new("Test", 100, 10, "Hz");
        assert_eq!(s.get_value(), 100);
    }

    #[test]
    fn label_returns_label() {
        let s = NumericSetting::new("Frequency", 500, 5, "Hz");
        assert_eq!(s.label(), "Frequency");
    }

    #[test]
    fn display_value_formats_correctly() {
        let s = NumericSetting::new("Frequency", 500, 5, "Hz");
        assert_eq!(s.display_value(), "Frequency: 500 Hz");
    }

    #[test]
    fn increment_adds_step() {
        let mut s = NumericSetting::new("Test", 100, 10, "Hz");
        s.increment();
        assert_eq!(s.get_value(), 110);
    }

    #[test]
    fn decrement_subtracts_step() {
        let mut s = NumericSetting::new("Test", 100, 10, "Hz");
        s.decrement();
        assert_eq!(s.get_value(), 90);
    }

    #[test]
    fn multiple_increments_accumulate() {
        let mut s = NumericSetting::new("Test", 0, 5, "Hz");
        for _ in 0..3 {
            s.increment();
        }
        assert_eq!(s.get_value(), 15);
    }

    #[test]
    fn increment_saturates_at_i64_max() {
        let mut s = NumericSetting::new("Test", i64::MAX, 1, "Hz");
        s.increment();
        assert_eq!(s.get_value(), i64::MAX);
    }

    #[test]
    fn decrement_saturates_at_i64_min() {
        let mut s = NumericSetting::new("Test", i64::MIN, 1, "Hz");
        s.decrement();
        assert_eq!(s.get_value(), i64::MIN);
    }
}
