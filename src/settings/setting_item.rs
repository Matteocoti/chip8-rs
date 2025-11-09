/// Defines the behavior for a generic item in a settings list.
#[typetag::serde]
pub trait SettingItem {
    /// Returns the display label for the setting.
    fn label(&self) -> &str;

    /// Returns the current numeric value.
    fn get_value(&self) -> i64;

    /// Returns the current value formatted for display.
    fn display_value(&self) -> String;

    /// Increments the setting's value.
    fn increment(&mut self);

    /// Decrements the setting's value.
    fn decrement(&mut self);
}
