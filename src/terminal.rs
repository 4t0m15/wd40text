use crate::Position;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{stdout, Write as _};

pub struct Size {
    pub width: u16,
    pub height: u16,
}

pub struct Terminal {
    size: Size,
}

impl Terminal {
    pub fn default() -> Result<Self, std::io::Error> {
        let size = terminal::size()?;
        terminal::enable_raw_mode()?;
        Ok(Self {
            size: Size {
                width: size.0,
                height: size.1.saturating_sub(2),
            },
        })
    }

    #[must_use] 
    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn clear_screen() {
        execute!(stdout(), terminal::Clear(ClearType::All)).unwrap();
    }

    pub fn cursor_position(position: &Position) {
        let x = position.x as u16;
        let y = position.y as u16;
        execute!(stdout(), cursor::MoveTo(x, y)).unwrap();
    }

    pub fn flush() -> Result<(), std::io::Error> {
        stdout().flush()
    }

    pub fn read_key() -> Result<KeyCode, std::io::Error> {
        loop {
            if let Event::Key(KeyEvent {
                code,
                modifiers: _,
                kind,
                state: _,
            }) = event::read()?
            {
                if kind == KeyEventKind::Press {
                    return Ok(code);
                }
            }
        }
    }

    pub fn read_key_with_modifiers() -> Result<(KeyCode, KeyModifiers), std::io::Error> {
        loop {
            if let Event::Key(KeyEvent {
                code,
                modifiers,
                kind,
                state: _,
            }) = event::read()?
            {
                if kind == KeyEventKind::Press {
                    return Ok((code, modifiers));
                }
            }
        }
    }

    pub fn cursor_hide() {
        execute!(stdout(), cursor::Hide).unwrap();
    }

    pub fn cursor_show() {
        execute!(stdout(), cursor::Show).unwrap();
    }

    pub fn clear_current_line() {
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine)).unwrap();
    }

    pub fn set_bg_color(color: Color) {
        execute!(stdout(), SetBackgroundColor(color)).unwrap();
    }

    pub fn reset_bg_color() {
        execute!(stdout(), SetBackgroundColor(Color::Reset)).unwrap();
    }

    pub fn set_fg_color(color: Color) {
        execute!(stdout(), SetForegroundColor(color)).unwrap();
    }

    pub fn reset_fg_color() {
        execute!(stdout(), SetForegroundColor(Color::Reset)).unwrap();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        terminal::disable_raw_mode().unwrap();
        Self::clear_screen();
        Self::cursor_show();
    }
}
