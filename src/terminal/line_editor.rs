use super::*;

#[derive(Clone)]
pub struct LineEditor {
    pub buffer: String,
    cursor: usize,
}

impl Flushable for LineEditor {}

impl LineEditor {
    pub fn new() -> Self {
        LineEditor {
            buffer: String::new(),
            cursor: 0,
        }
    }

    pub fn move_cursor_left(&mut self, count: usize) {
        if self.cursor < count || count == 0 {
            self.flush_stdout();
            return;
        }
        self.cursor -= count;
        self.move_cursor(LEFT, count);
    }

    pub fn move_cursor_right(&mut self, count: usize) {
        if self.cursor + count > self.buffer.len() || count == 0 {
            self.flush_stdout();
            return;
        }
        self.cursor += count;
        self.move_cursor(RIGHT, count);
    }

    fn move_cursor(&mut self, direction: u8, count: usize) {
        print!(
            "{}{}{}{}",
            char::from(ESCAPE),
            char::from(BRACKET),
            count,
            char::from(direction)
        );
        self.flush_stdout();
    }

    fn print_remains(&mut self) {
        let substr: &str = &self.buffer.as_str()[self.cursor..];
        print!("{substr}");
        self.cursor = self.buffer.len();
    }

    pub fn print_char(&mut self, c: u8) {
        self.buffer.insert(self.cursor, char::from(c));
        self.print_remains();
        self.move_cursor_left(self.buffer.len() - self.cursor);
    }

    fn wipe_back(&mut self, count: usize) {
        for _ in 0..count {
            print!("{}", ' ');
            self.cursor += 1;
        }
    }

    pub fn print_line(&mut self, prompt: &'static str) {
        print!("{} {}", prompt, self.buffer);
        self.cursor = self.buffer.len();
    }

    pub fn clear_line(&self, prompt_len: usize) {
        let total_len = self.buffer.len() + prompt_len + 1;
        print!("{}", '\r');
        for _ in 0..total_len {
            print!("{}", ' ');
        }
        print!("{}", '\r');
    }

    pub fn delete_char(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.move_cursor_left(1);
        let before = self.cursor;
        self.buffer.remove(self.cursor);
        self.print_remains();
        self.wipe_back(1);
        self.move_cursor_left(self.cursor - before);
    }
}
