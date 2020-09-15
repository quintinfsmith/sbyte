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
    pub fn insert_to_register(&mut self, character: String) {
        let mut pair = self.register.split_at(self.cursor_offset);
        self.register = format!("{}{}{}", pair.0, character.as_str(), pair.1).to_string();
    }

    pub fn remove_from_register(&mut self) {
        let mut tmp_register: Vec<u8> = self.register.bytes().collect();
        tmp_register.remove(self.cursor_offset);
        self.register = std::str::from_utf8(tmp_register.as_slice()).unwrap().to_string();
    }

    pub fn apply_register(&mut self) -> Option<String> {
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
        let mut index = self.history.len() - 1;
        self.get_command(index)
    }

    pub fn backspace(&mut self) {
        if self.cursor_offset != 0 {
            let new_offset = self.cursor_offset - 1;
            self.set_cursor_offset(new_offset);
            self.remove_from_register();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.register.len() == 0
    }

    pub fn get_register(&self) -> String {
        self.register.clone()
    }

    pub fn clear_register(&mut self) {
        self.set_register("".to_string());
    }

    pub fn set_register(&mut self, new_register: String) {
        self.register = new_register.clone();
        self.cursor_offset = new_register.len();
    }

    pub fn get_cursor_offset(&self) -> usize {
        self.cursor_offset
    }

}
