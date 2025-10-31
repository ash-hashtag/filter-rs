#![allow(unused)]

mod command;
pub mod double_linked_list;
mod lines;
mod main_pane;
mod new_scroll;
mod pages;
mod rc_str;
mod scroll_view;
mod sync_child;

use std::{
    io::{Stdout, Write},
    time::{Duration, Instant},
};

use command::{CommandBuilder, CommandType};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use main_pane::{draw_space_menu, main_pane_draw, main_pane_with_page_scroll_draw};
use new_scroll::{main2, PageScrollState};
use pages::Page;
use ratatui::prelude::CrosstermBackend;
use scroll_view::ScrollState;

const REDRAW_MILLIS_FRAME_TIME: u64 = 64;

// #[tokio::main]
fn main() -> anyhow::Result<()> {
    init_logger();
    start_ratatui()?;
    Ok(())
}

fn start_ratatui() -> anyhow::Result<()> {
    let term = ratatui::init();
    if let Err(err) = run_ratatui(term) {
        log::error!("{:?}", err);
    }
    ratatui::restore();
    Ok(())
}

#[derive(Debug, Copy, Clone)]
enum TuiMode {
    Normal,
    Command,
}

pub struct ErrorTimer {
    error: String,
    start: Instant,
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

fn run_ratatui(mut term: ratatui::Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let child_args = get_child_args();
    let title = child_args.join(" ");

    let (stdout_tx, mut stdout_rx) = std::sync::mpsc::channel();
    let (child_stdin_tx, child_stdin_rx) = std::sync::mpsc::channel();

    let mut child_handle =
        sync_child::spawn_child_process(&child_args, Some(stdout_tx), None, Some(child_stdin_rx))?;
    let mut child_spawn_instant = Instant::now();

    let mut current_width = 0u16;
    let mut current_height = 0u16;

    let mut page_scroll_state = PageScrollState::default();
    page_scroll_state.auto_scroll = true;

    let mut child_exited = false;
    let mut is_space_toggled = false;
    let mut cmd_builder = CommandBuilder::default();
    let mut error_timer = ErrorTimer::new("");

    loop {
        let poll_duration = Duration::from_millis(REDRAW_MILLIS_FRAME_TIME);
        error_timer.check(Duration::from_secs(2));

        if event::poll(poll_duration)? {
            let event = crossterm::event::read()?;
            match event {
                Event::Resize(width, height) => {
                    log::info!(
                        "Resized from {}x{} to {}x{}",
                        current_width,
                        current_height,
                        width,
                        height
                    );

                    current_width = width;
                    current_height = height;
                }

                Event::Key(key_event) => match key_event.code {
                    KeyCode::Esc => {
                        page_scroll_state.auto_scroll = true;
                        is_space_toggled = false;
                        cmd_builder.clear();
                    }

                    KeyCode::Backspace => {
                        if !matches!(cmd_builder.cmd_type, CommandType::None) {
                            cmd_builder.cmd.pop();
                        }
                    }
                    KeyCode::Enter => {
                        log::info!("Applying command {:?}", cmd_builder);

                        match cmd_builder.cmd_type {
                            CommandType::JumpTo => {
                                if let Ok(line_number) = cmd_builder.cmd.parse::<usize>() {
                                    page_scroll_state.apply_queue(
                                        new_scroll::InstructionQueue::JumpTo(line_number),
                                    );
                                } else {
                                    error_timer = ErrorTimer::new(format!(
                                        "Unable to parse line number {}",
                                        cmd_builder.cmd
                                    ));
                                }
                            }
                            CommandType::Search => {
                                let search_for = &cmd_builder.cmd;
                            }
                            _ => {
                                log::warn!("unimplemented command type");
                            }
                        };

                        cmd_builder.clear();
                    }
                    KeyCode::Char(c) => {
                        if !matches!(cmd_builder.cmd_type, CommandType::None) {
                            cmd_builder.cmd.push(c);
                        } else {
                            if is_space_toggled {
                                match c {
                                    's' => {
                                        is_space_toggled = false;
                                        cmd_builder.cmd_type = CommandType::Search;
                                    }
                                    'r' => {
                                        is_space_toggled = false;
                                        cmd_builder.cmd_type = CommandType::Regex;
                                    }
                                    'i' => {
                                        is_space_toggled = false;
                                        cmd_builder.cmd_type = CommandType::Ignore;
                                    }
                                    ':' => {
                                        is_space_toggled = false;
                                        cmd_builder.cmd_type = CommandType::JumpTo;
                                    }
                                    'c' => {
                                        is_space_toggled = false;
                                        cmd_builder.clear();
                                    }
                                    ' ' => {
                                        is_space_toggled = !is_space_toggled;
                                    }
                                    _ => {}
                                }
                            } else {
                                match c {
                                    ' ' => {
                                        is_space_toggled = !is_space_toggled;
                                    }
                                    'n' => {
                                        page_scroll_state.show_line_numbers =
                                            !page_scroll_state.show_line_numbers;
                                    }

                                    'j' => {
                                        page_scroll_state.auto_scroll = false;
                                        page_scroll_state
                                            .apply_queue(new_scroll::InstructionQueue::Up);
                                    }
                                    'k' => {
                                        page_scroll_state.auto_scroll = false;
                                        page_scroll_state
                                            .apply_queue(new_scroll::InstructionQueue::Down);
                                    }
                                    '/' | ':' => {}
                                    _ => {
                                        if !child_exited {
                                            log::info!("Sending {c} to child process");
                                            child_stdin_tx.send(c as u8)?;
                                        }
                                    }
                                };
                            }
                        }

                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            match c {
                                'c' => {}
                                'q' => {
                                    break;
                                }
                                'g' => {}
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            };
        }

        if !child_exited {
            loop {
                match stdout_rx.try_recv() {
                    Ok(s) => {
                        // app_state.add_line(&s);
                        page_scroll_state.add_line(&s);
                    }
                    Err(err) => {
                        match err {
                            std::sync::mpsc::TryRecvError::Empty => {}
                            std::sync::mpsc::TryRecvError::Disconnected => {
                                log::warn!("child stdout disconnected");
                                child_exited = true;

                                let exit_status = child_handle.join().unwrap();
                                page_scroll_state.add_line(&format!(
                                    "Child exited with {} and time took {:?}",
                                    exit_status,
                                    child_spawn_instant.elapsed()
                                ));
                            }
                        };
                        break;
                    }
                }
            }
        }

        term.draw(|frame| {
            main_pane_with_page_scroll_draw(
                frame,
                &title,
                &mut page_scroll_state,
                &cmd_builder,
                &error_timer.error,
            );

            if is_space_toggled {
                draw_space_menu(frame);
            }
        })?;
    }

    // log::info!("Final lines {:?}", lines);

    Ok(())
}

fn get_child_args() -> Vec<String> {
    let args = std::env::args();
    let child_args = args.skip(1).collect::<Vec<_>>();
    if child_args.is_empty() {
        panic!("No child process mentioned");
    }

    return child_args;
}

fn init_logger() {
    use log::LevelFilter;
    use std::fs::File;

    let target = Box::new(File::create("/tmp/filter-log.txt").expect("Can't create file"));

    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(target))
        .filter(None, LevelFilter::Debug)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {}:{}] {}",
                // Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
}
