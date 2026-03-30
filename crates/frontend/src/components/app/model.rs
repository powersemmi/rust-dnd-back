/// Which floating window is currently in focus (highest z-order).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveWindow {
    None,
    Chat,
    Scenes,
    Tokens,
    Settings,
    Statistics,
    Voting,
}
