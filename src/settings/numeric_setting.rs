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
