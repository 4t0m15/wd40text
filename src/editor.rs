use crate::Document;
use crate::Row;
use crate::Terminal;
use core::time::Duration;
use crossterm::event::KeyCode;
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
    command_buffer: Option<String>,
    last_keys: Vec<char>,
    pending_save_command: Option<String>,
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
        let mut initial_status =
            String::from("Good Luck, have fun! Type i.: to enter command mode.");
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
            command_buffer: None,
            last_keys: Vec::new(),
            pending_save_command: None,
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
                &None,
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
    fn execute_command(&mut self, command: &str) {
        match command.trim() {
            "help" | "h" => {
                self.status_message = StatusMessage::from(
                    "Commands: :w=save | :q=quit | :wq=save&quit | :help".to_owned(),
                );
            }
            "w" | "save" => {
                if self.document.file_name.is_some() {
                    if self.document.save().is_ok() {
                        self.status_message =
                            StatusMessage::from("File saved successfully.".to_owned());
                    } else {
                        self.status_message = StatusMessage::from("Error writing file!".to_owned());
                    }
                } else {
                    // Prompt for filename
                    self.pending_save_command = Some("w".to_owned());
                    self.command_buffer = Some(String::new());
                    self.status_message = StatusMessage::from("Save as: ".to_owned());
                }
            }
            "q!" | "quit!" => {
                // Force quit: discard unsaved changes and exit immediately
                self.should_quit = true;
            }
            "q" | "quit" => {
                if self.document.is_dirty() {
                    self.status_message = StatusMessage::from(
                        "File has unsaved changes! Use :wq to save and quit, or :q! to quit without saving.".to_owned(),
                    );
                } else {
                    self.should_quit = true;
                }
            }
            "wq" => {
                if self.document.file_name.is_some() {
                    if self.document.save().is_ok() {
                        self.should_quit = true;
                    } else {
                        self.status_message = StatusMessage::from("Error writing file!".to_owned());
                    }
                } else {
                    // Prompt for filename then save and quit
                    self.pending_save_command = Some("wq".to_owned());
                    self.command_buffer = Some(String::new());
                    self.status_message = StatusMessage::from("Save as: ".to_owned());
                }
            }
            _ => {
                self.status_message = StatusMessage::from(format!("Unknown command: :{}", command));
            }
        }
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let pressed_key = Terminal::read_key()?;

        // Handle command buffer first (highest priority)
        if let Some(ref mut buffer) = self.command_buffer {
            match pressed_key {
                KeyCode::Enter => {
                    let input = buffer.clone();
                    // clear the command buffer since we're processing it now
                    self.command_buffer = None;

                    // If there's a pending save command, treat this input as the filename
                    if let Some(pending_cmd) = self.pending_save_command.take() {
                        let filename = input.trim();
                        if !filename.is_empty() {
                            self.document.file_name = Some(filename.to_owned());
                            if self.document.save().is_ok() {
                                self.status_message =
                                    StatusMessage::from(format!("File saved as: {}", filename));
                                if pending_cmd == "wq" {
                                    self.should_quit = true;
                                }
                            } else {
                                self.status_message =
                                    StatusMessage::from("Error writing file!".to_owned());
                            }
                        } else {
                            self.status_message =
                                StatusMessage::from("No filename provided.".to_owned());
                        }
                        self.last_keys.clear();
                    } else {
                        // No pending special prompt — this is a normal command
                        self.execute_command(&input);
                        self.last_keys.clear();
                    }
                }
                KeyCode::Esc => {
                    // Cancel any active command or pending prompt
                    self.command_buffer = None;
                    self.pending_save_command = None;
                    self.status_message = StatusMessage::from("Command cancelled".to_owned());
                    self.last_keys.clear();
                }
                KeyCode::Backspace => {
                    buffer.pop();
                }
                KeyCode::Char(c) => {
                    buffer.push(c);
                }
                _ => (),
            }
            self.scroll();
            return Ok(());
        }

        // Handle keypresses
        match pressed_key {
            KeyCode::Enter => {
                self.document.insert(&self.cursor_position, '\n');
                self.cursor_position.x = 0;
                self.cursor_position.y = self.cursor_position.y.saturating_add(1);
                self.last_keys.clear();
            }
            KeyCode::Char(c) => {
                // Track last keys for command sequence
                self.last_keys.push(c);
                if self.last_keys.len() > 3 {
                    self.last_keys.remove(0);
                }

                // Check for i.:  sequence to enter command mode
                if self.last_keys.len() >= 3
                    && self.last_keys[self.last_keys.len() - 3] == 'i'
                    && self.last_keys[self.last_keys.len() - 2] == '.'
                    && self.last_keys[self.last_keys.len() - 1] == ':'
                {
                    // Remove the "i.:" that was just typed
                    for _ in 0..3 {
                        if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                            self.move_cursor(KeyCode::Left);
                            self.document.delete(&self.cursor_position);
                        }
                    }

                    // Enter command mode
                    self.command_buffer = Some(String::new());
                    self.status_message = StatusMessage::from("-- COMMAND MODE --".to_owned());
                    self.last_keys.clear();
                } else {
                    self.document.insert(&self.cursor_position, c);
                    self.move_cursor(KeyCode::Right);
                }
            }
            KeyCode::Delete => {
                self.document.delete(&self.cursor_position);
                self.last_keys.clear();
            }
            KeyCode::Backspace => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(KeyCode::Left);
                    self.document.delete(&self.cursor_position);
                }
                self.last_keys.clear();
            }
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::End
            | KeyCode::Home => {
                self.move_cursor(pressed_key);
                self.last_keys.clear();
            }
            _ => {
                self.last_keys.clear();
            }
        }

        self.scroll();
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
            if self.pending_save_command.is_some() {
                print!("Save as: {}", buffer);
            } else {
                print!(":{}", buffer);
            }
        } else {
            let message = &self.status_message;
            if message.time.elapsed() < Duration::new(5, 0) {
                let mut text = message.text.clone();
                text.truncate(self.terminal.size().width as usize);
                print!("{text}");
            }
        }
    }
}

fn die(e: std::io::Error) {
    Terminal::clear_screen();
    panic!("{}", e);
}
