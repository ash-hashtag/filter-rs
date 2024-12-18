mod child;
mod filter_scroll_view;
mod main_pane;
mod pages;
mod rc_str;
mod search_pane;
mod state;

use std::io::{Stdout, Write};

use child::{spawn_child_process, ChildHandle};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use filter_scroll_view::main_pane_draw;
use ratatui::prelude::CrosstermBackend;
use tokio::sync::mpsc::UnboundedSender;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

fn run_ratatui(mut term: ratatui::Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let (stdout_tx, mut stdout_rx) = tokio::sync::mpsc::unbounded_channel();
    let (stderr_tx, _stderr_rx) = tokio::sync::mpsc::unbounded_channel();
    let child_args = get_child_args();
    let (_child_handle, child_stdin_tx) = start_child(child_args, stdout_tx, stderr_tx)?;

    let mut current_width = 0u16;
    let mut current_height = 0u16;

    let mut state = filter_scroll_view::State::new(0, String::new(), TuiMode::Normal, true);

    loop {
        if event::poll(std::time::Duration::from_millis(60))? {
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
                        if matches!(state.get_mode(), TuiMode::Command) {
                            state.set_mode(TuiMode::Normal);
                        }
                        state.set_auto_scroll(true);
                    }

                    KeyCode::Backspace => {
                        if matches!(state.get_mode(), TuiMode::Command) {
                            state.command.pop();
                        }
                    }
                    KeyCode::Enter => {
                        if matches!(state.get_mode(), TuiMode::Command) {
                            // execute command
                            log::info!("Executing command {}", state.command);
                            state.command.clear();
                            state.set_mode(TuiMode::Normal);
                        }
                    }
                    KeyCode::Char(c) => {
                        match state.get_mode() {
                            TuiMode::Normal => match c {
                                'j' => state.go_up(),
                                'k' => state.go_down(),
                                '/' => {
                                    state.set_mode(TuiMode::Command);
                                    if state.command.is_empty() {
                                        state.command.push('/');
                                    }
                                }
                                _ => {
                                    child_stdin_tx.send(c as u8)?;
                                    log::info!("Sending {c} to child process");
                                }
                            },
                            TuiMode::Command => {
                                state.command.push(c);
                            }
                        }

                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            match c {
                                'c' => {
                                    state.command.clear();
                                    state.set_mode(TuiMode::Normal);
                                }
                                'd' => {
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

        match stdout_rx.try_recv() {
            Ok(mut s) => {
                s.push('\n');
                state.add_content(s.as_str());
            }
            Err(err) => match err {
                tokio::sync::mpsc::error::TryRecvError::Empty => {}
                tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                    log::warn!("child stdout disconnected");
                    break;
                }
            },
        }

        if stdout_rx.is_closed() {
            log::error!("child stdout closed");
            break;
        }

        term.draw(|frame| main_pane_draw(frame, &mut state))?;
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

fn start_child(
    args: Vec<String>,
    stdout_sender: UnboundedSender<String>,
    stderr_sender: UnboundedSender<String>,
) -> anyhow::Result<(ChildHandle, UnboundedSender<u8>)> {
    use tokio::sync::mpsc;
    log::info!("Starting child process with args {:?}", args);
    let (stdin_tx, stdin_rx) = mpsc::unbounded_channel::<u8>();
    let child_handle = spawn_child_process(&args, stdout_sender, stderr_sender, stdin_rx)?;

    Ok((child_handle, stdin_tx))
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
