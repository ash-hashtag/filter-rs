#![allow(unused)]

mod command;
mod main_pane;
mod new_scroll;
mod pages;
mod rc_str;
mod scroll_view;
mod sync_child;

use std::io::{Stdout, Write};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use main_pane::{main_pane_draw, main_pane_with_page_scroll_draw};
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

fn run_ratatui(mut term: ratatui::Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let child_args = get_child_args();
    let title = child_args.join(" ");

    // let (stdout_tx, mut stdout_rx) = tokio::sync::mpsc::unbounded_channel();
    // let (stderr_tx, _stderr_rx) = tokio::sync::mpsc::unbounded_channel();
    // let (_child_handle, child_stdin_tx) = start_child(child_args, stdout_tx, stderr_tx)?;

    let (stdout_tx, mut stdout_rx) = std::sync::mpsc::channel();
    let (child_stdin_tx, child_stdin_rx) = std::sync::mpsc::channel();
    let _child_handle =
        sync_child::spawn_child_process(&child_args, Some(stdout_tx), None, Some(child_stdin_rx))?;

    let mut current_width = 0u16;
    let mut current_height = 0u16;

    let mut page_scroll_state = PageScrollState::default();
    page_scroll_state.auto_scroll = true;

    let mut app_state = scroll_view::AppState::new(Page::new(), TuiMode::Normal);
    let mut main_scroll_state = ScrollState::default();
    let mut search_scroll_state = ScrollState::default();
    main_scroll_state.set_auto_scroll(true);
    // search_scroll_state.set_max_scroll_offset();
    search_scroll_state.set_auto_scroll(true);

    let mut child_exited = false;

    loop {
        if event::poll(std::time::Duration::from_millis(REDRAW_MILLIS_FRAME_TIME))? {
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
                        if matches!(app_state.get_mode(), TuiMode::Command) {
                            app_state.set_mode(TuiMode::Normal);
                            app_state.command.clear();
                            app_state.reset_search();
                            search_scroll_state.reset_scroll_position();
                        }
                        main_scroll_state.set_auto_scroll(true);
                        search_scroll_state.set_auto_scroll(true);
                        page_scroll_state.auto_scroll = true;
                    }

                    KeyCode::Backspace => {
                        if matches!(app_state.get_mode(), TuiMode::Command) {
                            app_state.command.pop();
                            app_state.reset_search();
                            // search_scroll_state.set_max_scroll_offset();
                            search_scroll_state.reset_scroll_position();
                        }
                    }
                    KeyCode::Enter => {
                        if matches!(app_state.get_mode(), TuiMode::Command) {
                            app_state.set_mode(TuiMode::Normal);
                        }
                    }
                    KeyCode::Char(c) => {
                        match app_state.get_mode() {
                            TuiMode::Normal => match c {
                                'n' => {
                                    app_state.show_line_numbers = !app_state.show_line_numbers;
                                }
                                'j' => {
                                    page_scroll_state.auto_scroll = false;
                                    page_scroll_state.apply_queue(new_scroll::InstructionQueue::Up);
                                    if app_state.is_in_search_mode() {
                                        search_scroll_state.go_up();
                                    } else {
                                        main_scroll_state.go_up();
                                    }
                                }
                                'k' => {
                                    page_scroll_state.auto_scroll = false;
                                    page_scroll_state
                                        .apply_queue(new_scroll::InstructionQueue::Down);
                                    if app_state.is_in_search_mode() {
                                        search_scroll_state.go_down();
                                    } else {
                                        main_scroll_state.go_down();
                                    }
                                }
                                '/' | ':' => {
                                    app_state.set_mode(TuiMode::Command);
                                    if app_state.command.is_empty() {
                                        app_state.command.push('/');
                                    }
                                }
                                _ => {
                                    if !child_exited {
                                        log::info!("Sending {c} to child process");
                                        child_stdin_tx.send(c as u8)?;
                                    }
                                }
                            },
                            TuiMode::Command => {
                                app_state.command.push(c);
                                app_state.reset_search();
                                search_scroll_state.reset_scroll_position();
                            }
                        }

                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            match c {
                                'c' => {
                                    app_state.command.clear();
                                    app_state.set_mode(TuiMode::Normal);
                                }
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
            match stdout_rx.try_recv() {
                Ok(s) => {
                    app_state.add_line(&s);
                    page_scroll_state.add_line(&s);
                }
                Err(err) => match err {
                    std::sync::mpsc::TryRecvError::Empty => {}
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        log::warn!("child stdout disconnected");
                        child_exited = true;
                    }
                },
            }
        }

        term.draw(|frame| {
            // if app_state.is_in_search_mode() {
            //     main_pane_draw(
            //         frame,
            //         title.as_str(),
            //         &mut app_state,
            //         &mut search_scroll_state,
            //     );
            // } else {
            //     main_pane_draw(
            //         frame,
            //         title.as_str(),
            //         &mut app_state,
            //         &mut main_scroll_state,
            //     );
            // }
            main_pane_with_page_scroll_draw(frame, &title, &mut page_scroll_state);
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
