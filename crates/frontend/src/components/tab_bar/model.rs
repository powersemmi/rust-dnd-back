/// Descriptor for a single tab entry in a tab bar.
#[derive(Clone)]
pub struct TabItem<T> {
    pub value: T,
    pub label: String,
    pub closable: bool,
}

impl<T> TabItem<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            closable: false,
        }
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tab_is_not_closable_by_default() {
        let tab = TabItem::new(42u32, "My Tab");
        assert!(!tab.closable);
        assert_eq!(tab.label, "My Tab");
    }

    #[test]
    fn closable_builder_sets_flag() {
        let tab = TabItem::new(1u32, "Tab").closable();
        assert!(tab.closable);
    }
}
