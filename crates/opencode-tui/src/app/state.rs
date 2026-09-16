#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AppState {
    #[default]
    Running,
    Exiting,
    Detaching,
    PromptFocused,
    DialogOpen,
    CommandPalette,
}
