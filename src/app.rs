use std::fs::File;
use std::io::{self, IsTerminal};
use std::path::PathBuf;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::alias::{filter_aliases, load_aliases, save_aliases, validate_alias_name, Alias};
use crate::history::{delete_entry, detect_history_file, load_history, HistoryEntry};
use crate::search::{SearchEngine, SearchResult};
use crate::ui;
use crate::Args;

pub const EXIT_CODE_EXECUTE: i32 = 10;

const INPUT_HEIGHT: u16 = 3;
const PREVIEW_HEIGHT: u16 = 8;
const HELP_HEIGHT: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Search,
    AliasCreate,
    AliasModify,
    Aliases,
}

pub struct ListNav {
    selected: usize,
    scroll_offset: usize,
    query: String,
}

impl ListNav {
    fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            query: String::new(),
        }
    }

    fn with_query(query: String) -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            query,
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    fn navigate_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn navigate_down(&mut self, max: usize) {
        if self.selected + 1 < max {
            self.selected += 1;
        }
    }

    fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(20);
    }

    fn page_down(&mut self, max: usize) {
        self.selected = (self.selected + 20).min(max.saturating_sub(1));
    }

    fn push_char(&mut self, c: char) {
        self.query.push(c);
    }

    fn pop_char(&mut self) {
        self.query.pop();
    }

    fn clear_query(&mut self) {
        self.query.clear();
    }

    fn clamp_selected(&mut self, len: usize) {
        if self.selected >= len {
            self.selected = len.saturating_sub(1);
        }
        self.scroll_offset = 0;
    }

    pub fn calculate_scroll(&mut self, visible_height: usize, total_items: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if visible_height > 0 && self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected.saturating_sub(visible_height - 1);
        }
        self.scroll_offset = self
            .scroll_offset
            .min(total_items.saturating_sub(visible_height));
    }
}

pub struct App {
    entries: Vec<HistoryEntry>,
    results: Vec<SearchResult>,
    search_engine: SearchEngine,
    search_nav: ListNav,

    should_quit: bool,
    selected_command: Option<String>,
    execute_immediately: bool,
    history_path: PathBuf,
    status_message: Option<String>,
    mode: AppMode,

    // Alias creation modal
    alias_input: String,
    alias_target_command: Option<String>,

    // Aliases tab
    aliases: Vec<Alias>,
    filtered_aliases: Vec<(usize, Alias)>,
    alias_nav: ListNav,
    alias_editing: Option<usize>,
    alias_edit_input: String,
    alias_modify_idx: Option<usize>,
    alias_modify_input: String,
}

impl App {
    pub fn new(
        entries: Vec<HistoryEntry>,
        history_path: PathBuf,
        initial_query: Option<String>,
        start_mode: AppMode,
    ) -> Self {
        let search_engine = SearchEngine::new();
        let query = initial_query.unwrap_or_default();
        let results = search_engine.search(&entries, &query);
        let aliases = load_aliases();
        let filtered_aliases = filter_aliases(&aliases, "");

        Self {
            entries,
            results,
            search_engine,
            search_nav: ListNav::with_query(query),
            should_quit: false,
            selected_command: None,
            execute_immediately: false,
            history_path,
            status_message: None,
            mode: start_mode,
            alias_input: String::new(),
            alias_target_command: None,
            aliases,
            filtered_aliases,
            alias_nav: ListNav::new(),
            alias_editing: None,
            alias_edit_input: String::new(),
            alias_modify_idx: None,
            alias_modify_input: String::new(),
        }
    }

    // --- Accessors for UI ---

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn search_nav(&self) -> &ListNav {
        &self.search_nav
    }

    pub fn results(&self) -> &[SearchResult] {
        &self.results
    }

    pub fn alias_nav(&self) -> &ListNav {
        &self.alias_nav
    }

    pub fn filtered_aliases(&self) -> &[(usize, Alias)] {
        &self.filtered_aliases
    }

    pub fn alias_editing(&self) -> Option<usize> {
        self.alias_editing
    }

    pub fn alias_edit_input(&self) -> &str {
        &self.alias_edit_input
    }

    pub fn alias_modify_input(&self) -> &str {
        &self.alias_modify_input
    }

    pub fn alias_modify_name(&self) -> Option<&str> {
        self.alias_modify_idx
            .and_then(|idx| self.aliases.get(idx))
            .map(|a| a.name.as_str())
    }

    pub fn alias_input(&self) -> &str {
        &self.alias_input
    }

    pub fn alias_target_command(&self) -> Option<&str> {
        self.alias_target_command.as_deref()
    }

    pub fn selected_command_preview(&self) -> Option<&str> {
        self.results
            .get(self.search_nav.selected)
            .map(|r| r.entry.command.as_str())
    }

    pub fn selected_alias_preview(&self) -> Option<&str> {
        self.filtered_aliases
            .get(self.alias_nav.selected)
            .map(|(_, a)| a.command.as_str())
    }

    pub fn prepare_render(&mut self, terminal_height: u16) {
        let visible = terminal_height
            .saturating_sub(INPUT_HEIGHT + PREVIEW_HEIGHT + HELP_HEIGHT + 2)
            as usize;
        match self.mode {
            AppMode::Search | AppMode::AliasCreate => {
                let total = self.results.len();
                self.search_nav.calculate_scroll(visible, total);
            }
            AppMode::Aliases | AppMode::AliasModify => {
                let total = self.filtered_aliases.len();
                self.alias_nav.calculate_scroll(visible, total);
            }
        }
    }

    // --- Internal methods ---

    fn update_search(&mut self) {
        self.results = self
            .search_engine
            .search(&self.entries, &self.search_nav.query);
        self.search_nav.clamp_selected(self.results.len());
    }

    fn update_alias_filter(&mut self) {
        self.filtered_aliases = filter_aliases(&self.aliases, &self.alias_nav.query);
        self.alias_nav.clamp_selected(self.filtered_aliases.len());
    }

    fn delete_selected(&mut self) {
        let Some(result) = self.results.get(self.search_nav.selected) else {
            return;
        };

        let command = result.entry.command.clone();
        let prev_selected = self.search_nav.selected;

        if let Err(e) = delete_entry(&self.history_path, &result.entry) {
            self.status_message = Some(format!("Delete failed: {}", e));
            return;
        }

        self.entries.retain(|e| e.command != command);
        self.results = self
            .search_engine
            .search(&self.entries, &self.search_nav.query);
        self.search_nav.selected = prev_selected.min(self.results.len().saturating_sub(1));
    }

    fn select_command(&mut self, execute: bool) {
        if let Some(result) = self.results.get(self.search_nav.selected) {
            self.selected_command = Some(result.entry.command.clone());
            self.execute_immediately = execute;
        }
        self.should_quit = true;
    }

    fn select_alias_command(&mut self, execute: bool) {
        if let Some((_, alias)) = self.filtered_aliases.get(self.alias_nav.selected) {
            self.selected_command = Some(alias.command.clone());
            self.execute_immediately = execute;
        }
        self.should_quit = true;
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match self.mode {
            AppMode::Search => self.handle_search_key(code, modifiers),
            AppMode::AliasCreate => self.handle_alias_create_key(code, modifiers),
            AppMode::AliasModify => self.handle_alias_modify_key(code, modifiers),
            AppMode::Aliases => self.handle_alias_key(code, modifiers),
        }
    }

    fn handle_search_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.status_message = None;
        match (code, modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => self.delete_selected(),
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                if let Some(result) = self.results.get(self.search_nav.selected) {
                    self.alias_target_command = Some(result.entry.command.clone());
                    self.alias_input.clear();
                    self.mode = AppMode::AliasCreate;
                }
            }
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.mode = AppMode::Aliases;
                self.update_alias_filter();
            }
            (KeyCode::Enter, _) => self.select_command(false),
            (KeyCode::Tab, _) => self.select_command(true),
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.search_nav.navigate_up();
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.search_nav.navigate_down(self.results.len());
            }
            (KeyCode::PageUp, _) => {
                self.search_nav.page_up();
            }
            (KeyCode::PageDown, _) => {
                self.search_nav.page_down(self.results.len());
            }
            (KeyCode::Backspace, _) => {
                self.search_nav.pop_char();
                self.update_search();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.search_nav.clear_query();
                self.update_search();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.search_nav.push_char(c);
                self.update_search();
            }
            _ => {}
        }
    }

    fn handle_alias_create_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Esc, _) => {
                self.alias_target_command = None;
                self.alias_input.clear();
                self.mode = AppMode::Search;
            }
            (KeyCode::Enter, _) => {
                let name = self.alias_input.trim().to_string();
                if let Err(msg) = validate_alias_name(&name) {
                    self.status_message = Some(msg.to_string());
                    return;
                }
                if self.aliases.iter().any(|a| a.name == name) {
                    self.status_message = Some("Alias name already exists".to_string());
                    return;
                }
                if let Some(command) = self.alias_target_command.take() {
                    self.aliases.push(Alias {
                        name: name.clone(),
                        command,
                    });
                    if let Err(e) = save_aliases(&self.aliases) {
                        self.status_message = Some(format!("Save failed: {}", e));
                    } else {
                        self.status_message = Some(format!("Alias '{}' saved", name));
                    }
                    self.update_alias_filter();
                }
                self.alias_input.clear();
                self.mode = AppMode::Search;
            }
            (KeyCode::Backspace, _) => {
                self.alias_input.pop();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.alias_input.clear();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT)
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' =>
            {
                self.alias_input.push(c);
            }
            _ => {}
        }
    }

    fn handle_alias_modify_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Esc, _) => {
                self.alias_modify_idx = None;
                self.alias_modify_input.clear();
                self.mode = AppMode::Aliases;
            }
            (KeyCode::Enter, _) => {
                let new_command = self.alias_modify_input.trim().to_string();
                if new_command.is_empty() {
                    self.status_message = Some("Command cannot be empty".to_string());
                    return;
                }
                if let Some(idx) = self.alias_modify_idx {
                    self.aliases[idx].command = new_command;
                    if let Err(e) = save_aliases(&self.aliases) {
                        self.status_message = Some(format!("Save failed: {}", e));
                    }
                    self.update_alias_filter();
                }
                self.alias_modify_idx = None;
                self.alias_modify_input.clear();
                self.mode = AppMode::Aliases;
            }
            (KeyCode::Backspace, _) => {
                self.alias_modify_input.pop();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.alias_modify_input.clear();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.alias_modify_input.push(c);
            }
            _ => {}
        }
    }

    fn handle_alias_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.status_message = None;

        // Handle inline rename mode
        if let Some(editing_orig_idx) = self.alias_editing {
            match (code, modifiers) {
                (KeyCode::Esc, _) => {
                    self.alias_editing = None;
                    self.alias_edit_input.clear();
                }
                (KeyCode::Enter, _) => {
                    let new_name = self.alias_edit_input.trim().to_string();
                    if let Err(msg) = validate_alias_name(&new_name) {
                        self.status_message = Some(msg.to_string());
                        return;
                    }
                    if self
                        .aliases
                        .iter()
                        .enumerate()
                        .any(|(i, a)| a.name == new_name && i != editing_orig_idx)
                    {
                        self.status_message = Some("Name already exists".to_string());
                        return;
                    }
                    self.aliases[editing_orig_idx].name = new_name;
                    if let Err(e) = save_aliases(&self.aliases) {
                        self.status_message = Some(format!("Save failed: {}", e));
                    }
                    self.alias_editing = None;
                    self.alias_edit_input.clear();
                    self.update_alias_filter();
                }
                (KeyCode::Backspace, _) => {
                    self.alias_edit_input.pop();
                }
                (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                    self.alias_edit_input.clear();
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT)
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' =>
                {
                    self.alias_edit_input.push(c);
                }
                _ => {}
            }
            return;
        }

        match (code, modifiers) {
            (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                self.mode = AppMode::Search;
            }
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                self.alias_nav.navigate_up();
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.alias_nav.navigate_down(self.filtered_aliases.len());
            }
            (KeyCode::PageUp, _) => {
                self.alias_nav.page_up();
            }
            (KeyCode::PageDown, _) => {
                self.alias_nav.page_down(self.filtered_aliases.len());
            }
            (KeyCode::Enter, _) => self.select_alias_command(false),
            (KeyCode::Tab, _) => self.select_alias_command(true),
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                if let Some((orig_idx, _)) = self.filtered_aliases.get(self.alias_nav.selected) {
                    let orig_idx = *orig_idx;
                    self.aliases.remove(orig_idx);
                    if let Err(e) = save_aliases(&self.aliases) {
                        self.status_message = Some(format!("Save failed: {}", e));
                    }
                    self.update_alias_filter();
                }
            }
            (KeyCode::Char('r'), KeyModifiers::NONE) if self.alias_nav.query.is_empty() => {
                if let Some((orig_idx, alias)) = self.filtered_aliases.get(self.alias_nav.selected)
                {
                    self.alias_editing = Some(*orig_idx);
                    self.alias_edit_input = alias.name.clone();
                }
            }
            (KeyCode::Char('m'), KeyModifiers::NONE) if self.alias_nav.query.is_empty() => {
                if let Some((orig_idx, alias)) = self.filtered_aliases.get(self.alias_nav.selected)
                {
                    self.alias_modify_idx = Some(*orig_idx);
                    self.alias_modify_input = alias.command.clone();
                    self.mode = AppMode::AliasModify;
                }
            }
            (KeyCode::Backspace, _) => {
                self.alias_nav.pop_char();
                self.update_alias_filter();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.alias_nav.clear_query();
                self.update_alias_filter();
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.alias_nav.push_char(c);
                self.update_alias_filter();
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

    let start_mode = if args.aliases {
        AppMode::Aliases
    } else {
        AppMode::Search
    };
    let mut app = App::new(entries, history_path, args.query, start_mode);
    let result = run_event_loop(&mut terminal, &mut app);

    // Cleanup terminal before any output
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    drop(terminal);

    result?;

    if let Some(ref command) = app.selected_command {
        copy_to_clipboard(command);
        print!("{}", command);
        // Add newline when running standalone (stdout is terminal),
        // but not when captured by ih() wrapper via $()
        if std::io::stdout().is_terminal() {
            println!();
        }
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
        app.prepare_render(frame.area().height);
        ui::render(frame, app);
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
            app.prepare_render(frame.area().height);
            ui::render(frame, app);
        })?;
    }

    Ok(())
}
