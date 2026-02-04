mod action;
mod app;
mod command;
mod main_pane;
mod new_scroll;
mod pages;
mod sync_child;

use app::App;
use std::io::Write;

// #[tokio::main]
fn main() -> anyhow::Result<()> {
    init_logger();
    start_ratatui()?;
    Ok(())
}

fn start_ratatui() -> anyhow::Result<()> {
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut term = ratatui::init();
    let mut app = App::new()?;
    let result = app.run(&mut term);

    ratatui::restore();
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;

    if let Err(err) = result {
        log::error!("{:?}", err);
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

fn init_logger() {
    use log::LevelFilter;
    use std::fs::File;

    if let Ok(log_file_path) = std::env::var("FILTER_LOG_FILE") {
        
    let target = Box::new(File::create(log_file_path).expect("Can't create file"));
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
 
}
