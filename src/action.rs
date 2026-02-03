use crate::command::CommandType;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    Resize(u16, u16),
    Tick,

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

    SearchNext,
    SearchPrev,

    // Communication
    SendToChild(char),

    // Toggles
    ToggleLineNumbers,
    ToggleAutoscroll,
}
