use anyhow::Context;
use oxigraph::{
    io::{RdfFormat, RdfParser},
    sparql::QueryResults,
    store::Store,
};
use ratatui::{
    backend::Backend,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Padding, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::{
    fs, io,
    path::{absolute, Path},
    time::Duration,
};

/// 應用程序之總體名理。
pub struct App {
    store: Store,
    mode: Mode,
    query: Query,
    quitting: bool,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let store = Store::new()?;
        let mode = Mode::Browse;
        let query = Query::new();
        let quitting = false;
        Ok(Self {
            store,
            mode,
            query,
            quitting,
        })
    }

    /// 啓動循環
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()> {
        loop {
            self.draw(terminal)?;
            self.handle_event()?;
            // 退出條件
            if self.quitting {
                break;
            }
        }
        Ok(())
    }

    /// 處理事件。
    fn handle_event(&mut self) -> anyhow::Result<()> {
        if event::poll(Duration::from_millis(16))? {
            // 目前只處理鍵盤事件，轉交 handle_key
            if let Event::Key(key) = event::read()? {
                self.handle_key(key)?;
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        // 只处理鍵盤按下事件
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        // 根據模式轉交不同處理。
        match self.mode {
            Mode::Query => self.handle_key_code_in_query_mode(key.code)?,
            Mode::Browse => self.handle_key_code_in_browse_mode(key.code)?,
        }

        Ok(())
    }

    /// 輸入模式下處理輸入按鍵。
    fn handle_key_code_in_query_mode(&mut self, code: KeyCode) -> anyhow::Result<()> {
        match code {
            // 退格，清除字
            KeyCode::Backspace => {
                self.query.pop();
            }
            // 回車，換行
            KeyCode::Enter => self.query.push('\n'),
            // 制表，切換模式
            KeyCode::Tab => self.switch_mode()?,
            // 輸入字符
            KeyCode::Char(ch) => self.query.push(ch),
            _ => {}
        };
        Ok(())
    }

    /// 瀏覽模式下處理輸入按鍵。
    fn handle_key_code_in_browse_mode(&mut self, code: KeyCode) -> anyhow::Result<()> {
        match code {
            // 切換模式
            KeyCode::Tab => self.switch_mode()?,
            KeyCode::Char('q') => self.quit(),
            _ => {}
        }
        Ok(())
    }

    /// 切換模式。
    fn switch_mode(&mut self) -> anyhow::Result<()> {
        match self.mode {
            Mode::Query => {
                self.mode = Mode::Browse;
            }
            Mode::Browse => self.mode = Mode::Query,
        }
        Ok(())
    }

    /// 設置退出狀態。
    fn quit(&mut self) {
        self.quitting = true;
    }

    /// 繪製終端。
    pub fn draw<B: Backend>(&self, terminal: &mut Terminal<B>) -> io::Result<()> {
        terminal.draw(|frame| {
            use Constraint::{Fill, Length};
            let layout = Layout::vertical([Length(self.query.height), Fill(1)]).split(frame.size());
            self.render_query(frame, layout[0]);
            self.render_browser(frame, layout[1]);
        })?;
        Ok(())
    }

    /// 渲染瀏覽部分
    fn render_browser(&self, frame: &mut Frame, rect: Rect) {
        let block = Block::bordered()
            .title("Explore".bold())
            .border_style(self.get_browser_style())
            .padding(Padding::horizontal(1));

        // 僅在查詢結果时
        if let Ok(QueryResults::Solutions(solutions)) = self.store.query(self.query.string.as_str())
        {
            let variables = solutions.variables().to_vec();

            let widths = [Constraint::Fill(1)].repeat(variables.len());
            let header = Row::new(variables.iter().map(|v| v.to_string()))
                .bold()
                .underlined();

            let mut rows = vec![];
            for s in solutions {
                if let Ok(s) = s {
                    rows.push(Row::new(
                        variables.iter().map(|v| s.get(v).unwrap().to_string()),
                    ));
                }
            }

            let table = Table::new(rows, widths)
                .column_spacing(1)
                .header(header)
                .block(block);

            frame.render_widget(table, rect);
        } else {
            frame.render_widget(Paragraph::new("NO RESULT").centered().block(block), rect);
        }
    }

    /// 渲染查詢部分
    fn render_query(&self, frame: &mut Frame, rect: Rect) {
        frame.render_widget(
            Paragraph::new(self.query.string.as_str()).block(
                Block::bordered()
                    .title("Query".bold())
                    .border_style(self.get_query_style()),
            ),
            rect,
        );
    }

    /// 瀏覽部分樣式。
    fn get_browser_style(&self) -> Style {
        let style = Style::default();
        match self.mode {
            Mode::Browse => style.fg(Color::Green),
            Mode::Query => style,
        }
    }

    /// 查詢部分樣式。
    fn get_query_style(&self) -> Style {
        let style = Style::default();
        match self.mode {
            Mode::Query => style.fg(Color::Green),
            Mode::Browse => style,
        }
    }

    /// 讀取 path
    pub fn load(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = absolute(path.as_ref())?;
        let iri = format!(
            "file://{}",
            path.to_str().context("Fail to convert path to string")?
        );
        let file = fs::read_to_string(&path)?;
        self.store.load_from_read(
            RdfParser::from_format(RdfFormat::Turtle).with_base_iri(&iri)?,
            file.as_bytes(),
        )?;
        Ok(())
    }
}

/// 應用有「瀏覽」和「輸入查詢」兩狀態。
/// 應用根據狀態決定將事件傳到哪裏。
pub enum Mode {
    Query,
    Browse,
}

/// 查詢字串。同時記錄其形狀，以减少計算成本。
pub struct Query {
    string: String,
    height: u16,
}

impl Query {
    /// 新建。默認高度為 3。
    pub fn new() -> Self {
        Self {
            string: "SELECT ?s ?p ?o WHERE { ?s ?p ?o }".to_string(),
            height: 3,
        }
    }

    /// 推入字符。根據是否 `\n` 計算形狀。
    pub fn push(&mut self, ch: char) {
        self.string.push(ch);
        // 若換行，則高益寬復。
        if ch == '\n' {
            self.height += 1;
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        let ch = self.string.pop();
        if let Some('\n') = ch {
            self.height -= 1;
        }
        ch
    }
}
