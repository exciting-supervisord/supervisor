extern crate libc;

mod ffi;
mod flushable;
mod line_editor;

use flushable::Flushable;
use line_editor::LineEditor;
use nix::errno;
use std::collections::VecDeque;

const UP: u8 = 0x41;
const DOWN: u8 = 0x42;
const LEFT: u8 = 0x44;
const RIGHT: u8 = 0x43;
const ENTER: u8 = 0x0a;
const ESCAPE: u8 = 0x1b;
const BRACKET: u8 = 0x5b;
const DELETE1: u8 = 0x33;
const DELETE2: u8 = 0x7e;
const KEY_HOME: u8 = 0x48;
const KEY_END: u8 = 0x46;
const BACKSPACE: u8 = 0x7f;

#[derive(Debug)]
enum LineState {
    CHAR,
    ESCAPE,
    ARROW,
    DELETE,
}

pub struct Terminal {
    history: VecDeque<LineEditor>,
    history_index: usize,
    prompt: &'static str,
}

impl Flushable for Terminal {}

impl Terminal {
    pub fn new(prompt: &'static str) -> Self {
        Terminal {
            history: Default::default(),
            history_index: 0,
            prompt,
        }
    }

    fn history_next(&mut self) {
        if self.history_index == 0 {
            return;
        }

        let prompt = self.prompt;
        let curr_line = &self.history[self.history_index];
        curr_line.clear_line(prompt.len());
        let next_line = &mut self.history[self.history_index - 1];
        next_line.print_line(prompt);
        self.history_index -= 1;
        self.flush_stdout();
    }

    fn history_prev(&mut self) {
        if self.history_index == self.history.len() - 1 {
            return;
        }

        let prompt = self.prompt;
        let curr_line = &self.history[self.history_index];
        curr_line.clear_line(prompt.len());
        let next_line = &mut self.history[self.history_index + 1];
        next_line.print_line(prompt);
        self.history_index += 1;
        self.flush_stdout();
    }

    fn record_history(&mut self) -> String {
        let index = self.history_index;
        let history = &mut self.history;
        if index != 0 {
            history[0] = history[index].clone();
        }
        if history[0].buffer.is_empty() {
            history.remove(0);
            "".to_string()
        } else {
            history[0].buffer.to_string()
        }
    }

    fn line_init(&mut self) {
        print!("{} ", self.prompt);
        self.flush_stdout();
        self.history.push_front(LineEditor::new());
        self.history_index = 0;
    }

    pub fn getline(&mut self) -> Result<String, errno::Errno> {
        self.line_init();
        let mut state = LineState::CHAR;
        loop {
            let ch = ffi::getch()?;
            // println!("{ch}");
            let line_editor = &mut self.history[self.history_index];

            match state {
                LineState::CHAR => match ch {
                    ENTER => break,
                    ESCAPE => state = LineState::ESCAPE,
                    BACKSPACE => line_editor.delete_char_prev(),
                    0x20..=0x7e => line_editor.print_char(ch),
                    _ => {}
                },
                LineState::ESCAPE => match ch {
                    BRACKET => state = LineState::ARROW,
                    ESCAPE => state = LineState::CHAR,
                    _ => {
                        line_editor.print_char(ch);
                        state = LineState::CHAR
                    }
                },
                LineState::ARROW => {
                    state = LineState::CHAR;
                    match ch {
                        UP => self.history_prev(),
                        DOWN => self.history_next(),
                        LEFT => line_editor.move_cursor_left(1),
                        RIGHT => line_editor.move_cursor_right(1),
                        DELETE1 => state = LineState::DELETE,
                        KEY_HOME => line_editor.move_cursor_home(),
                        KEY_END => line_editor.move_cursor_end(),
                        _ => {}
                    }
                }
                LineState::DELETE => {
                    state = LineState::CHAR;
                    match ch {
                        DELETE2 => line_editor.delete_char_curr(),
                        _ => {}
                    }
                }
            }
        }
        println!();
        Ok(self.record_history())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}
