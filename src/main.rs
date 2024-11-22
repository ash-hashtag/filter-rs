pub mod child;
mod rc_str;
mod state;

use std::{
    io::{Stdout, Write},
    sync::{Arc, Mutex},
};

use child::{spawn_child_process, ChildHandle, Receiver};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode},
    ExecutableCommand, QueueableCommand,
};
use state::State;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const PREFIX_KEY: char = 'g';

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

            if let Event::Key(key_event) = event {
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
                            clear_command_prompt(&stdout)?;

                            if !buf.is_empty() {
                                stdout.queue(cursor::MoveToRow(0))?;
                                stdout.queue(cursor::MoveToColumn(0))?;
                                println!("\n> {}", buf);
                                stdout.queue(cursor::MoveToColumn(0))?;
                                let cmd = buf.trim();
                                if cmd == "exit" {
                                    break;
                                }

                                // if let Some(child_cmd) = cmd.strip_prefix("exec ") {
                                //     child_stdin_tx = execute_cmd(child_cmd).ok();
                                // }

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
        }
        stdout.flush()?;
    }

    Ok(())
}

fn clear_command_prompt(mut stdout: &Stdout) -> anyhow::Result<()> {
    let (rows, _) = terminal::size()?;
    stdout.queue(cursor::MoveToRow(rows))?;
    stdout.queue(cursor::MoveToColumn(0))?;
    stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
    Ok(())
}

fn execute_cmd(args: Vec<String>, state: Arc<Mutex<State>>) -> anyhow::Result<UnboundedSender<u8>> {
    use tokio::sync::mpsc;

    let (stdout_tx, stdout_rx) = mpsc::unbounded_channel::<String>();
    let (stderr_tx, stderr_rx) = mpsc::unbounded_channel::<String>();
    let (stdin_tx, stdin_rx) = mpsc::unbounded_channel::<u8>();
    let _child_handle = spawn_child_process(&args, stdout_tx, stderr_tx, stdin_rx)?;
    tokio::spawn(print_reading_lines(stdout_rx, state));
    Ok(stdin_tx)
}

async fn print_reading_lines(mut receiver: UnboundedReceiver<String>, state: Arc<Mutex<State>>) {
    while let Some(buf) = receiver.recv().await {
        if let Err(err) = state.lock().unwrap().add_line(&buf) {
            eprintln!("can't add line {}", err);
            break;
        }
    }
}
