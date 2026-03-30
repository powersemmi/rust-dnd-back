/// Descriptor for a single menu button.
#[derive(Clone, Debug)]
pub struct MenuButtonConfig {
    pub icon: &'static str,
    pub label_key: &'static str,
    pub hotkey_key: &'static str,
    pub has_notification: bool,
    pub notification_count: u32,
}
