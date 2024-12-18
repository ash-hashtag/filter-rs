mod child;
mod filter_scroll_view;
mod main_pane;
mod pages;
mod rc_str;
mod scroll_view;
mod scroll_view_state;
mod search_pane;
mod state;

use std::{
    io::{Stdout, Write},
    sync::{Arc, Mutex},
};

use child::spawn_child_process;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use filter_scroll_view::main_pane_draw;
use ratatui::prelude::CrosstermBackend;
use state::State;
use tokio::sync::mpsc::UnboundedSender;

const PREFIX_KEY: char = 'g';

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    start_ratatui()?;
    Ok(())
}

pub enum TerminalMode {
    Default,
    Normal,
}

fn start_ratatui() -> anyhow::Result<()> {
    let term = ratatui::init();
    let _ = run_ratatui(term);
    ratatui::restore();
    Ok(())
}

#[derive(Debug, Copy, Clone)]
enum TuiMode {
    Normal,
    Insert,
}

fn run_ratatui(mut term: ratatui::Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let (stdout_tx, stdout_rx) = tokio::sync::mpsc::unbounded_channel();
    let (stderr_tx, stderr_rx) = tokio::sync::mpsc::unbounded_channel();
    let child_args = get_child_args();
    let child_stdin_tx = start_child(child_args, stdout_tx, stderr_tx)?;
    let mut lines = vec![String::new()];
    let mut vertical_position = 0usize;
    let mut current_width = 0u16;
    let mut current_height = 0u16;
    let mut mode = TuiMode::Normal;
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
                        if matches!(mode, TuiMode::Insert) {
                            mode = TuiMode::Normal;
                        }
                    }

                    KeyCode::Backspace => {
                        if matches!(mode, TuiMode::Insert) {
                            lines.last_mut().unwrap().pop();
                        }
                    }
                    KeyCode::Enter => {
                        if matches!(mode, TuiMode::Insert) {
                            lines.push(String::new());
                        }
                    }
                    KeyCode::Char(c) => {
                        match mode {
                            TuiMode::Normal => match c {
                                'k' => vertical_position += 1,
                                'j' => {
                                    if vertical_position > 0 {
                                        vertical_position -= 1;
                                    }
                                }
                                'i' => {
                                    mode = TuiMode::Insert;
                                }
                                _ => {}
                            },
                            TuiMode::Insert => lines.last_mut().unwrap().push(c),
                        }

                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            match c {
                                'c' | 'd' => {
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

        term.draw(|frame| main_pane_draw(frame, &lines, vertical_position, mode))?;
    }

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

async fn start_tui() -> anyhow::Result<()> {
    let instant = std::time::Instant::now();
    let args = std::env::args();
    let child_args = args.skip(1).collect::<Vec<_>>();
    if child_args.is_empty() {
        panic!("No child process mentioned");
    }
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    let start_positon = cursor::position()?;
    stdout.queue(terminal::EnterAlternateScreen)?;
    stdout.queue(terminal::Clear(terminal::ClearType::All))?;
    stdout.queue(cursor::Hide)?;
    stdout.queue(cursor::MoveToRow(0))?;
    stdout.flush()?;
    let _ = tokio::task::spawn_blocking(|| run(child_args)).await;
    stdout.queue(terminal::LeaveAlternateScreen)?;
    stdout.queue(cursor::MoveTo(start_positon.0, start_positon.1))?;
    stdout.queue(cursor::Show)?;
    disable_raw_mode()?;
    println!("spent {:?}s", instant.elapsed());
    Ok(())
}

fn run(child_args: Vec<String>) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout();
    let mut command_mode = false;
    let state = Arc::new(Mutex::new(State::new()));
    let mut buf = String::new();
    let child_stdin_tx: UnboundedSender<u8> = execute_cmd(child_args, state.clone())?;
    loop {
        let mut key_consumed = command_mode;
        if event::poll(std::time::Duration::from_millis(60))? {
            let event = crossterm::event::read()?;

            match event {
                Event::Resize(width, height) => {}
                Event::Key(key_event) => {
                    match key_event.code {
                        KeyCode::Backspace => {
                            if command_mode {
                                if !buf.is_empty() {
                                    buf.pop();
                                    stdout.queue(cursor::MoveLeft(1))?;
                                    stdout.write(&[' ' as u8])?;
                                    stdout.queue(cursor::MoveLeft(1))?;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if command_mode {
                                command_mode = false;
                                clear_command_prompt(&mut stdout)?;

                                if !buf.is_empty() {
                                    stdout.queue(cursor::MoveToRow(0))?;
                                    stdout.queue(cursor::MoveToColumn(0))?;
                                    println!("\n> {}", buf);
                                    stdout.queue(cursor::MoveToColumn(0))?;
                                    let cmd = buf.trim();
                                    if cmd == "exit" {
                                        break;
                                    }
                                    buf.clear();
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            if command_mode {
                                print!("{}", c);
                                buf.push(c);
                            } else {
                                print!("{}", c);
                            }

                            if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                match c {
                                    'c' | 'd' => {
                                        // key_consumed = true;
                                        break;
                                    }
                                    PREFIX_KEY => {
                                        command_mode = true;
                                        let (_, rows) = terminal::size()?;
                                        stdout.queue(cursor::MoveToRow(rows))?;
                                        stdout.queue(cursor::MoveToColumn(0))?;
                                        print!("command: ");
                                        key_consumed = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }

                    if !key_consumed {
                        match key_event.code {
                            KeyCode::Char(c) => {
                                child_stdin_tx.send(c as u8)?;
                            }
                            KeyCode::Enter => {
                                child_stdin_tx.send('\n' as u8)?;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            };
        }
        stdout.flush()?;
    }

    Ok(())
}

fn clear_command_prompt<T>(writer: &mut T) -> anyhow::Result<()>
where
    T: std::io::Write,
{
    let (rows, _) = terminal::size()?;
    writer.queue(cursor::MoveToRow(rows))?;
    writer.queue(cursor::MoveToColumn(0))?;
    writer.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
    Ok(())
}

fn start_child(
    args: Vec<String>,
    stdout_sender: UnboundedSender<String>,
    stderr_sender: UnboundedSender<String>,
) -> anyhow::Result<UnboundedSender<u8>> {
    use tokio::sync::mpsc;
    let (stdin_tx, stdin_rx) = mpsc::unbounded_channel::<u8>();
    let _child_handle = spawn_child_process(&args, stdout_sender, stderr_sender, stdin_rx)?;
    Ok(stdin_tx)
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
