use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    Terminal,
};
use std::io::{self, stdout, Stdout};

/// 初始化終端。
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    // 於 panic 時恢復終端
    std::panic::set_hook(Box::new(|info| {
        restore_terminal();
        eprintln!("{}", info);
    }));
    ratatui::Terminal::new(CrosstermBackend::new(stdout()))
}

/// 恢復終端。
pub fn restore_terminal() {
    stdout().execute(LeaveAlternateScreen).unwrap();
    disable_raw_mode().unwrap();
}
