use crate::Document;
use crate::Row;
use crate::Terminal;
use core::time::Duration;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::style::Color;
use std::env;
use std::time::Instant;

const STATUS_FG_COLOR: Color = Color::Rgb {
    r: 63,
    g: 63,
    b: 63,
};
const STATUS_BG_COLOR: Color = Color::Rgb {
    r: 239,
    g: 239,
    b: 239,
};
const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

#[derive(PartialEq, Copy, Clone)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Default, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}
impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            time: Instant::now(),
            text: message,
        }
    }
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_message: StatusMessage,
    quit_times: u8,
    highlighted_word: Option<String>,
    command_buffer: Option<String>,
}

impl Editor {
    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(error);
            }
            if self.should_quit {
                break;
            }
            if let Err(error) = self.process_keypress() {
                die(error);
            }
        }
    }
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = String::from(
            "HELP: Ctrl-F = find | Ctrl-S = save | :q = quit | :q! = force quit | :help = show help again",
        );

        let document = if let Some(file_name) = args.get(1) {
            let doc = Document::open(file_name);
            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("ERR: Could not open file: {file_name}");
                Document::default()
            }
        } else {
            Document::default()
        };

        Self {
            should_quit: false,
            terminal: Terminal::default().expect("Failed to initialize terminal"),
            document,
            cursor_position: Position::default(),
            offset: Position::default(),
            status_message: StatusMessage::from(initial_status),
            quit_times: QUIT_TIMES,
            highlighted_word: None,
            command_buffer: None,
        }
    }

    fn refresh_screen(&mut self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());
        if self.should_quit {
            Terminal::clear_screen();
            println!("Come Again!.\r");
        } else {
            self.document.highlight(
                &self.highlighted_word,
                Some(
                    self.offset
                        .y
                        .saturating_add(self.terminal.size().height as usize),
                ),
            );
            self.draw_rows();
            self.draw_status_bar();
            self.draw_message_bar();
            if let Some(ref buffer) = self.command_buffer {
                Terminal::cursor_position(&Position {
                    x: buffer.len() + 1,
                    y: self.terminal.size().height as usize + 1,
                });
            } else {
                Terminal::cursor_position(&Position {
                    x: self.cursor_position.x.saturating_sub(self.offset.x),
                    y: self.cursor_position.y.saturating_sub(self.offset.y),
                });
            }
        }
        Terminal::cursor_show();
        Terminal::flush()
    }
    fn save(&mut self) {
        if self.document.file_name.is_none() {
            // Ask for a base file name first (without extension)
            let base_name = self.prompt("name: ", |_, _, _, _| {}).unwrap_or(None);
            if base_name.is_none() {
                self.status_message = StatusMessage::from("aborted".to_owned());
                return;
            }
            let mut base_name = base_name.unwrap();

            // If the user already provided an extension in the name, use it as-is.
            // Otherwise, ask for a preferred extension and append it.
            if !base_name.contains('.') {
                let ext = self
                    .prompt("(txt/docx/odt): ", |_, _, _, _| {})
                    .unwrap_or(None);
                let chosen_ext = match ext.as_deref() {
                    Some(ext) => {
                        let mut e = ext.trim().to_ascii_lowercase();
                        if e.starts_with('.') {
                            e = e.trim_start_matches('.').to_string();
                        }
                        match e.as_str() {
                            "txt" | "docx" | "odt" => e,
                            _ => "txt".to_string(),
                        }
                    }
                    None => "txt".to_string(), // default to txt if empty/cancelled
                };
                base_name.push('.');
                base_name.push_str(&chosen_ext);
            }
            self.document.file_name = Some(base_name);
        }

        if self.document.save().is_ok() {
            self.status_message = StatusMessage::from("File saved successfully.".to_owned());
        } else {
            self.status_message = StatusMessage::from("Error writing file!".to_owned());
        }
    }
    fn search(&mut self) {
        let old_position = self.cursor_position.clone();
        let mut direction = SearchDirection::Forward;
        let query = self
            .prompt(
                "Search (ESC to cancel, Arrows to navigate): ",
                |editor, key, _mods, query| {
                    let mut moved = false;
                    match key {
                        KeyCode::Right | KeyCode::Down => {
                            direction = SearchDirection::Forward;
                            editor.move_cursor(KeyCode::Right);
                            moved = true;
                        }
                        KeyCode::Left | KeyCode::Up => direction = SearchDirection::Backward,
                        _ => direction = SearchDirection::Forward,
                    }
                    if let Some(position) =
                        editor
                            .document
                            .find(query, &editor.cursor_position, direction)
                    {
                        editor.cursor_position = position;
                        editor.scroll();
                    } else if moved {
                        editor.move_cursor(KeyCode::Left);
                    }
                    editor.highlighted_word = Some(query.clone());
                },
            )
            .unwrap_or(None);

        if query.is_none() {
            self.cursor_position = old_position;
            self.scroll();
        }
        self.highlighted_word = None;
    }
    fn execute_command(&mut self, command: &str) {
        match command.trim() {
            "help" | "h" | "?" => {
                self.status_message = StatusMessage::from(
                    "HELP: Ctrl-F=find | Ctrl-S=save | Enter=newline | ESC=cancel | :w=write | :q=quit | :q!=force quit | :wq=write+quit | :help=show help | Save-as will ask for .txt/.docx/.odt"
                        .to_owned(),
                );
            }
            "q" => {
                if self.quit_times > 0 && self.document.is_dirty() {
                    self.status_message = StatusMessage::from(format!(
                        "WARNING! File has unsaved changes. Use :q! to force quit or save first."
                    ));
                    self.quit_times -= 1;
                } else {
                    self.should_quit = true;
                }
            }
            "q!" => {
                self.should_quit = true;
            }
            "w" => {
                self.save();
            }
            "wq" => {
                self.save();
                self.should_quit = true;
            }
            _ => {
                self.status_message = StatusMessage::from(format!("Unknown command: {}", command));
            }
        }
    }
    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let (pressed_key, modifiers) = Terminal::read_key_with_modifiers()?;
        match pressed_key {
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => self.save(),
            KeyCode::Char('f')
                if modifiers.contains(KeyModifiers::CONTROL) && self.command_buffer.is_none() =>
            {
                self.search()
            }
            KeyCode::Enter => {
                if let Some(buffer) = self.command_buffer.take() {
                    self.execute_command(&buffer);
                } else {
                    self.document.insert(&self.cursor_position, '\n');
                    self.cursor_position.x = 0;
                    self.cursor_position.y = self.cursor_position.y.saturating_add(1);
                }
            }
            KeyCode::Char(':') if self.command_buffer.is_none() => {
                self.command_buffer = Some(String::new());
            }
            KeyCode::Char(c) => {
                if let Some(ref mut buffer) = self.command_buffer {
                    buffer.push(c);
                } else {
                    self.document.insert(&self.cursor_position, c);
                    self.move_cursor(KeyCode::Right);
                }
            }
            KeyCode::Esc => {
                self.command_buffer = None;
            }
            KeyCode::Delete => self.document.delete(&self.cursor_position),
            KeyCode::Backspace => {
                if let Some(ref mut buffer) = self.command_buffer {
                    buffer.pop();
                } else if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(KeyCode::Left);
                    self.document.delete(&self.cursor_position);
                }
            }
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::End
            | KeyCode::Home => self.move_cursor(pressed_key),
            _ => (),
        }
        self.scroll();
        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_message = StatusMessage::from(String::new());
        }
        Ok(())
    }
    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;
        let offset = &mut self.offset;
        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_sub(height).saturating_add(1);
        }
        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }
    fn move_cursor(&mut self, key: KeyCode) {
        let terminal_height = self.terminal.size().height as usize;
        let Position { mut y, mut x } = self.cursor_position;
        let height = self.document.len();
        let mut width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        match key {
            KeyCode::Up => y = y.saturating_sub(1),
            KeyCode::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            }
            KeyCode::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            }
            KeyCode::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            }
            KeyCode::PageUp => {
                y = if y > terminal_height {
                    y.saturating_sub(terminal_height)
                } else {
                    0
                }
            }
            KeyCode::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y.saturating_add(terminal_height)
                } else {
                    height
                }
            }
            KeyCode::Home => x = 0,
            KeyCode::End => x = width,
            _ => (),
        }
        width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };
        if x > width {
            x = width;
        }

        self.cursor_position = Position { x, y }
    }
    fn draw_welcome_message(&self) {
        let mut welcome_message = format!("wd40 -- version {VERSION}");
        let width = self.terminal.size().width as usize;
        let len = welcome_message.len();
        #[expect(clippy::arithmetic_side_effects, clippy::integer_division)]
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);
        println!("{welcome_message}\r");
    }
    pub fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end = self.offset.x.saturating_add(width);
        let row = row.render(start, end);
        println!("{row}\r");
    }
    #[expect(clippy::integer_division, clippy::arithmetic_side_effects)]
    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();
            if let Some(row) = self
                .document
                .row(self.offset.y.saturating_add(terminal_row as usize))
            {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcome_message();
            } else {
                println!("~\r");
            }
        }
    }
    fn draw_status_bar(&self) {
        let mut status;
        let width = self.terminal.size().width as usize;
        let modified_indicator = if self.document.is_dirty() {
            " (modified)"
        } else {
            ""
        };

        let mut path_display = "[No Name]".to_owned();
        if let Some(name) = &self.document.file_name {
            path_display = name.clone();
        }
        status = format!(
            "{} - {} lines{}",
            path_display,
            self.document.len(),
            modified_indicator
        );

        let line_indicator = format!(
            "{} | {}/{} | {} chars",
            self.document.file_type(),
            self.cursor_position.y.saturating_add(1),
            self.document.len(),
            self.document.char_count()
        );
        #[expect(clippy::arithmetic_side_effects)]
        let len = status.len() + line_indicator.len();
        status.push_str(&" ".repeat(width.saturating_sub(len)));
        status = format!("{status}{line_indicator}");
        status.truncate(width);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        Terminal::set_fg_color(STATUS_FG_COLOR);
        println!("{status}\r");
        Terminal::reset_fg_color();
        Terminal::reset_bg_color();
    }
    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        if let Some(ref buffer) = self.command_buffer {
            print!(":{}", buffer);
        } else {
            let message = &self.status_message;
            if message.time.elapsed() < Duration::new(5, 0) {
                let mut text = message.text.clone();
                text.truncate(self.terminal.size().width as usize);
                print!("{text}");
            }
        }
    }
    fn prompt<C>(&mut self, prompt: &str, mut callback: C) -> Result<Option<String>, std::io::Error>
    where
        C: FnMut(&mut Self, KeyCode, KeyModifiers, &String),
    {
        let mut result = String::new();
        loop {
            self.status_message = StatusMessage::from(format!("{prompt}{result}"));
            self.refresh_screen()?;
            let (key, modifiers) = Terminal::read_key_with_modifiers()?;
            match key {
                KeyCode::Backspace => result.truncate(result.len().saturating_sub(1)),
                KeyCode::Enter => break,
                KeyCode::Char(c) => {
                    result.push(c);
                }
                KeyCode::Esc => {
                    result.truncate(0);
                    break;
                }
                _ => (),
            }
            callback(self, key, modifiers, &result);
        }
        self.status_message = StatusMessage::from(String::new());
        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(result))
    }
}

fn die(e: std::io::Error) {
    Terminal::clear_screen();
    panic!("{}", e);
}
