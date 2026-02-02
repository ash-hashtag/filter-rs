use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::*;
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::{
    action::Action,
    command::{Command, CommandBuilder, CommandType},
    new_scroll::{InstructionQueue, PageScrollState},
    pages::Pages,
    sync_child,
};

pub struct ErrorTimer {
    pub error: String,
    pub start: Instant,
}

impl ErrorTimer {
    pub fn check(&mut self, duration: Duration) {
        if self.start.elapsed() > duration {
            self.error.clear();
        }
    }

    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            start: Instant::now(),
        }
    }
}

pub struct App {
    pub pages: Arc<RwLock<Pages>>,

    pub scroll_state: PageScrollState,
    pub cmd_builder: CommandBuilder,
    pub is_space_toggled: bool,
    pub error_timer: ErrorTimer,
    pub current_size: (u16, u16),
    pub should_quit: bool,

    pub child_handle: Option<sync_child::ChildHandle>,
    pub child_pid: Option<u32>,
    pub stdout_rx: std::sync::mpsc::Receiver<String>,
    pub child_stdin_tx: std::sync::mpsc::Sender<u8>,
    pub child_spawn_instant: Instant,
    pub child_exited: bool,
    pub title: String,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let child_args = crate::get_child_args();
        let title = child_args.join(" ");

        let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
        let (child_stdin_tx, child_stdin_rx) = std::sync::mpsc::channel();

        // Note: spawn_child_process implementation might need to be checked if it returns handle and pid
        // Assuming it's the same as in main.rs
        let child_handle = sync_child::spawn_child_process(
            &child_args,
            Some(stdout_tx),
            None,
            Some(child_stdin_rx),
        )?;

        let pages = Arc::new(RwLock::new(Pages::new(100, 30)));
        let mut scroll_state = PageScrollState::new(pages.clone());
        scroll_state.set_auto_scroll(true);

        Ok(Self {
            pages: pages,
            scroll_state,
            cmd_builder: CommandBuilder::default(),
            is_space_toggled: false,
            error_timer: ErrorTimer::new(""),
            current_size: (0, 0),
            should_quit: false,

            child_handle: Some(child_handle),
            child_pid: None, // If needed
            stdout_rx,
            child_stdin_tx,
            child_spawn_instant: Instant::now(),
            child_exited: false,
            title,
        })
    }

    pub fn run(
        &mut self,
        term: &mut ratatui::Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> anyhow::Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(64); // REDRAW_MILLIS_FRAME_TIME

        loop {
            if self.should_quit {
                break;
            }

            self.poll_child();
            term.draw(|frame| {
                crate::main_pane::main_pane_with_page_scroll_draw(frame, self);
                if self.is_space_toggled {
                    crate::main_pane::draw_space_menu(frame);
                }
            })?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                let event = crossterm::event::read()?;
                log::info!("Event: {:?}", event);
                let action = self.get_action(event);

                if let Some(action) = action {
                    self.update(action)?;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.update(Action::Tick)?;
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn get_action(&self, event: Event) -> Option<Action> {
        match event {
            Event::Resize(w, h) => Some(Action::Resize(w, h)),
            Event::Key(key) => {
                if key.kind == event::KeyEventKind::Press {
                    self.handle_key_event(key)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_key_event(&self, key: event::KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::ClearCommand),
            KeyCode::Backspace => {
                if !matches!(self.cmd_builder.cmd_type, CommandType::None) {
                    Some(Action::DeleteBackCommand)
                } else {
                    None
                }
            }
            KeyCode::Enter => Some(Action::ExecuteCommand),
            KeyCode::Char(c) => {
                if !matches!(self.cmd_builder.cmd_type, CommandType::None) {
                    Some(Action::TypeCommand(c))
                } else {
                    if self.is_space_toggled {
                        match c {
                            's' => Some(Action::Command(CommandType::Search)),
                            'r' => Some(Action::Command(CommandType::Regex)),
                            'i' => Some(Action::Command(CommandType::Ignore)),
                            ':' => Some(Action::Command(CommandType::JumpTo)),
                            'c' => Some(Action::ClearCommand),
                            'q' => Some(Action::Quit),
                            ' ' => Some(Action::ToggleSpaceMenu),
                            _ => None,
                        }
                    } else {
                        match c {
                            ' ' => Some(Action::ToggleSpaceMenu),
                            'n' => Some(Action::ToggleLineNumbers),
                            'j' => Some(Action::ScrollDown),
                            'k' => Some(Action::ScrollUp),
                            'q' => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    Some(Action::Quit)
                                } else {
                                    None
                                }
                            }
                            // Handle other keys or forward to child
                            _ => {
                                // For now we don't return an Action for child input, expecting side effect or we add an Action::SendToChild
                                // But get_action is &self, so we return Action::SendToChild(c)
                                // Wait, I need Action::SendToChild
                                Some(Action::SendToChild(c))
                            }
                        }
                    }
                }
            }
            _ => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('q') => Some(Action::Quit),
                        _ => None,
                    }
                } else {
                    None
                }
            }
        }
    }

    pub fn update(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::Resize(w, h) => {
                self.current_size = (w, h);
                self.scroll_state.apply_queue(InstructionQueue::Resize {
                    width: w as usize,
                    height: h as usize,
                });
            }
            Action::Tick => {
                self.error_timer.check(Duration::from_secs(2));
                self.poll_child();
            }
            Action::ToggleSpaceMenu => {
                self.is_space_toggled = !self.is_space_toggled;
            }
            Action::ClearCommand => {
                self.cmd_builder.clear();
                self.is_space_toggled = false;
            }
            Action::Command(cmd_type) => {
                self.is_space_toggled = false;
                self.cmd_builder.cmd_type = cmd_type;
            }
            Action::TypeCommand(c) => {
                self.cmd_builder.cmd.push(c);
            }
            Action::DeleteBackCommand => {
                self.cmd_builder.cmd.pop();
            }
            Action::ExecuteCommand => {
                self.execute_command();
            }
            Action::ScrollUp => {
                self.scroll_state.set_auto_scroll(false);
                self.scroll_state.apply_queue(InstructionQueue::Up);
            }
            Action::ScrollDown => {
                self.scroll_state.set_auto_scroll(false);
                self.scroll_state.apply_queue(InstructionQueue::Down);
            }
            Action::ToggleLineNumbers => {
                self.scroll_state
                    .set_show_line_numbers(!self.scroll_state.show_line_numbers());
            }
            Action::JumpTo(line_number) => {
                self.scroll_state
                    .apply_queue(InstructionQueue::JumpTo(line_number));
            }
            Action::SendToChild(c) => {
                if !self.child_exited {
                    // log::info!("Sending {c} to child process");
                    self.child_stdin_tx.send(c as u8)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_command(&mut self) {
        log::info!("Applying command {:?}", self.cmd_builder);
        match self.cmd_builder.cmd_type {
            CommandType::JumpTo => {
                if let Ok(line_number) = self.cmd_builder.cmd.parse::<usize>() {
                    self.scroll_state
                        .apply_queue(InstructionQueue::JumpTo(line_number));
                } else {
                    self.error_timer = ErrorTimer::new(format!(
                        "Unable to parse line number {}",
                        self.cmd_builder.cmd
                    ));
                }
                self.cmd_builder.clear();
            }
            CommandType::Search => {
                // Logic from main.rs
            }
            _ => {
                log::warn!("unimplemented command type");
            }
        }
    }

    fn poll_child(&mut self) {
        if !self.child_exited {
            loop {
                match self.stdout_rx.try_recv() {
                    Ok(s) => {
                        self.pages.write().unwrap().add_line(&s);
                    }
                    Err(err) => {
                        match err {
                            std::sync::mpsc::TryRecvError::Empty => {}
                            std::sync::mpsc::TryRecvError::Disconnected => {
                                log::warn!("child stdout disconnected");
                                self.child_exited = true;

                                if let Some(mut handle) = self.child_handle.take() {
                                    let exit_status = handle.join().unwrap();
                                    self.pages.write().unwrap().add_line(&format!(
                                        "Child exited with {} and time took {:?}",
                                        exit_status,
                                        self.child_spawn_instant.elapsed()
                                    ));
                                }
                            }
                        };
                        break;
                    }
                }
            }
        }
    }
}
