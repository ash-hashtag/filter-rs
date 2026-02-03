use crate::command::CommandType;
use crossterm::event::KeyEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    Resize(u16, u16),
    Tick,
    Render,
    Key(KeyEvent),

    // Mode transitions
    EnterCommandMode,
    EnterNormalMode,

    // Command specific
    ToggleSpaceMenu,
    ClearCommand,
    Command(CommandType),
    TypeCommand(char),
    DeleteBackCommand,
    ExecuteCommand,

    // Navigation
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    JumpTo(usize),
    SearchNext,
    SearchPrev,

    // Communication
    SendToChild(char),

    // Toggles
    ToggleLineNumbers,
    ToggleAutoscroll,
}
