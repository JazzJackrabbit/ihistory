use std::fs::File;
use std::io;
use std::path::PathBuf;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};

use crate::history::{delete_entry, detect_history_file, load_history, HistoryEntry};
use crate::search::{SearchEngine, SearchResult};
use crate::ui::UI;
use crate::Args;

pub const EXIT_CODE_EXECUTE: i32 = 10;

pub struct App {
    entries: Vec<HistoryEntry>,
    query: String,
    results: Vec<SearchResult>,
    selected: usize,
    scroll_offset: usize,
    search_engine: SearchEngine,
    ui: UI,
    list_state: ListState,
    should_quit: bool,
    selected_command: Option<String>,
    execute_immediately: bool,
    history_path: PathBuf,
    status_message: Option<String>,
}

impl App {
    pub fn new(
        entries: Vec<HistoryEntry>,
        history_path: PathBuf,
        initial_query: Option<String>,
    ) -> Self {
        let search_engine = SearchEngine::new();
        let query = initial_query.unwrap_or_default();
        let results = search_engine.search(&entries, &query);

        Self {
            entries,
            query,
            results,
            selected: 0,
            scroll_offset: 0,
            search_engine,
            ui: UI::new(),
            list_state: ListState::default(),
            should_quit: false,
            selected_command: None,
            execute_immediately: false,
            history_path,
            status_message: None,
        }
    }

    fn update_search(&mut self) {
        self.results = self.search_engine.search(&self.entries, &self.query);
        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
        self.scroll_offset = 0;
    }

    fn delete_selected(&mut self) {
        let Some(result) = self.results.get(self.selected) else {
            return;
        };

        let command = result.entry.command.clone();
        let prev_selected = self.selected;

        if let Err(e) = delete_entry(&self.history_path, &result.entry) {
            self.status_message = Some(format!("Delete failed: {}", e));
            return;
        }

        self.entries.retain(|e| e.command != command);
        self.results = self.search_engine.search(&self.entries, &self.query);
        self.selected = prev_selected.min(self.results.len().saturating_sub(1));
    }

    fn select_command(&mut self, execute: bool) {
        if let Some(result) = self.results.get(self.selected) {
            self.selected_command = Some(result.entry.command.clone());
            self.execute_immediately = execute;
        }
        self.should_quit = true;
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.status_message = None;
        match (code, modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => self.delete_selected(),
            (KeyCode::Enter, _) => self.select_command(false),
            (KeyCode::Tab, _) => self.select_command(true),
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.selected = self.selected.saturating_sub(1);
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                if self.selected + 1 < self.results.len() {
                    self.selected += 1;
                }
            }
            (KeyCode::PageUp, _) => {
                self.selected = self.selected.saturating_sub(20);
            }
            (KeyCode::PageDown, _) => {
                self.selected = (self.selected + 20).min(self.results.len().saturating_sub(1));
            }
            (KeyCode::Backspace, _) => {
                self.query.pop();
                self.update_search();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.query.clear();
                self.update_search();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.query.push(c);
                self.update_search();
            }
            _ => {}
        }
    }
}

pub fn run(args: Args) -> Result<i32, Box<dyn std::error::Error>> {
    let history_path = args.file.or_else(detect_history_file).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not find history file. Please specify one with --file",
        )
    })?;

    let entries = load_history(&history_path, args.limit)?;
    if entries.is_empty() {
        return Err("No history entries found".into());
    }

    // Use /dev/tty so TUI works inside $() subshells
    let mut tty = File::options().read(true).write(true).open("/dev/tty")?;

    enable_raw_mode()?;
    execute!(tty, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(tty);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(entries, history_path, args.query);
    let result = run_event_loop(&mut terminal, &mut app);

    // Cleanup terminal before any output
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    drop(terminal);

    result?;

    if let Some(ref command) = app.selected_command {
        copy_to_clipboard(command);
        print!("{}", command);
        std::io::Write::flush(&mut std::io::stdout())?;
    }

    Ok(if app.execute_immediately {
        EXIT_CODE_EXECUTE
    } else {
        0
    })
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<File>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    terminal.draw(|frame| {
        app.scroll_offset = app.ui.render(
            frame,
            &app.query,
            &app.results,
            app.selected,
            app.scroll_offset,
            &mut app.list_state,
            app.status_message.as_deref(),
        );
    })?;

    loop {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                app.handle_key(key.code, key.modifiers);
            }
            Event::Resize(_, _) => {
                // Redraw on resize
            }
            _ => continue, // Skip other events without redrawing
        }

        if app.should_quit {
            break;
        }

        terminal.draw(|frame| {
            app.scroll_offset = app.ui.render(
                frame,
                &app.query,
                &app.results,
                app.selected,
                app.scroll_offset,
                &mut app.list_state,
                app.status_message.as_deref(),
            );
        })?;
    }

    Ok(())
}
