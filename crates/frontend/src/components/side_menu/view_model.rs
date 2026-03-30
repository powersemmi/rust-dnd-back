use leptos::prelude::*;

/// Reactive state for the collapsible side menu.
#[derive(Clone, Copy)]
pub struct SideMenuViewModel {
    pub is_open: RwSignal<bool>,
}

impl SideMenuViewModel {
    pub fn new(is_open: RwSignal<bool>) -> Self {
        Self { is_open }
    }

    pub fn toggle(&self) {
        self.is_open.update(|open| *open = !*open);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    #[test]
    fn toggle_opens_closed_menu() {
        let owner = Owner::new();
        owner.with(|| {
            let is_open = RwSignal::new(false);
            let vm = SideMenuViewModel::new(is_open);
            vm.toggle();
            assert!(vm.is_open.get_untracked());
        });
    }

    #[test]
    fn toggle_closes_open_menu() {
        let owner = Owner::new();
        owner.with(|| {
            let is_open = RwSignal::new(true);
            let vm = SideMenuViewModel::new(is_open);
            vm.toggle();
            assert!(!vm.is_open.get_untracked());
        });
    }

    #[test]
    fn double_toggle_returns_to_initial() {
        let owner = Owner::new();
        owner.with(|| {
            let is_open = RwSignal::new(false);
            let vm = SideMenuViewModel::new(is_open);
            vm.toggle();
            vm.toggle();
            assert!(!vm.is_open.get_untracked());
        });
    }
}
