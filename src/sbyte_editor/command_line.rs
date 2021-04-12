use std::cmp::min;

pub struct CommandLine {
    history: Vec<String>,
    register: String,
    cursor_offset: usize
}

impl CommandLine {
    pub fn new() -> CommandLine {
        CommandLine {
            history: Vec::new(),
            register: "".to_string(),
            cursor_offset: 0
        }
    }

    pub fn set_cursor_offset(&mut self, index: usize) {
        self.cursor_offset = min(self.register.len(), index);
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        self.cursor_offset = min(self.register.len(), self.cursor_offset + 1);
    }

    pub fn insert_to_register(&mut self, chunk: &str) {
        let pair = self.register.split_at(self.cursor_offset);
        self.register = format!("{}{}{}", pair.0, chunk, pair.1).to_string();
    }

    pub fn remove_from_register(&mut self) -> Option<char> {
        if self.cursor_offset >= self.register.len() {
            None
        } else {
            Some(self.register.remove(self.cursor_offset))
        }
    }

    pub fn fetch_register(&mut self) -> Option<String> {
        let output = self.register.clone();
        self.history.push(self.register.clone());
        self.register = "".to_string();
        Some(output)
    }

    pub fn get_command(&mut self, index: usize) -> Option<String> {
        match self.history.get(index) {
            Some(cmd) => {
                Some((*cmd).to_string())
            }
            None => {
                None
            }
        }
    }

    pub fn get_last_command(&mut self) -> Option<String> {
        let index = self.history.len() - 1;
        self.get_command(index)
    }

    pub fn backspace(&mut self) -> Option<char> {
        if self.cursor_offset != 0 {
            let new_offset = self.cursor_offset - 1;
            self.set_cursor_offset(new_offset);
            self.remove_from_register()
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.register.len() == 0
    }

    pub fn get_register(&self) -> String {
        self.register.clone()
    }

    pub fn clear_register(&mut self) {
        self.set_register("");
    }

    pub fn set_register(&mut self, new_register: &str) {
        self.register = new_register.to_string();
        self.cursor_offset = new_register.len();
    }

    pub fn get_cursor_offset(&self) -> usize {
        self.cursor_offset
    }

}
