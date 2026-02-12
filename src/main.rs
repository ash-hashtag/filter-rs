mod action;
mod app;
mod command;
mod main_pane;
mod new_scroll;
mod pages;
mod sync_child;

use app::App;
use clap::Parser;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Command and arguments to run
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    child_args: Vec<String>,

    /// Maximum buffer size (e.g., 10MB, 1GB). If set, it will be divided into 64KB pages.
    #[arg(long, value_parser = parse_size)]
    max_buffer_size: Option<usize>,

    /// Number of pages in the circular buffer
    #[arg(long, default_value_t = 32)]
    pages_count: usize,

    /// Size of each page in the circular buffer (e.g., 64KB, 1MB)
    #[arg(long, value_parser = parse_size, default_value = "64KB")]
    page_size: usize,
}

fn parse_size(s: &str) -> Result<usize, String> {
    let s = s.to_uppercase();
    if s.ends_with("KB") {
        s[..s.len() - 2]
            .parse::<usize>()
            .map(|n| n * 1024)
            .map_err(|e| e.to_string())
    } else if s.ends_with("MB") {
        s[..s.len() - 2]
            .parse::<usize>()
            .map(|n| n * 1024 * 1024)
            .map_err(|e| e.to_string())
    } else if s.ends_with("GB") {
        s[..s.len() - 2]
            .parse::<usize>()
            .map(|n| n * 1024 * 1024 * 1024)
            .map_err(|e| e.to_string())
    } else if s.ends_with('B') {
        s[..s.len() - 1].parse::<usize>().map_err(|e| e.to_string())
    } else {
        s.parse::<usize>().map_err(|e| e.to_string())
    }
}

// #[tokio::main]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    init_logger();
    start_ratatui(args)?;
    Ok(())
}

fn start_ratatui(args: Args) -> anyhow::Result<()> {
    let (pages_count, page_size) = if let Some(max_buffer_size) = args.max_buffer_size {
        let page_size = 64 * 1024;
        let pages_count = (max_buffer_size + page_size - 1) / page_size;
        (pages_count, page_size)
    } else {
        (args.pages_count, args.page_size)
    };

    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut term = ratatui::init();
    let mut app = App::new(args.child_args, pages_count, page_size)?;
    let result = app.run(&mut term);

    ratatui::restore();
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;

    if let Err(err) = result {
        log::error!("{:?}", err);
    }
    Ok(())
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
