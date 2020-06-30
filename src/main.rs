use asciibox::RectManager;
use terminal_size::{Width, Height, terminal_size};

pub enum EditorError {
    OutofRange
}

struct Cursor {
    offset: usize,
    length: usize
}

impl Cursor {
    pub fn set_length(&mut self, new_length: usize) {
        self.length = new_length;
    },

    pub fn set_offset(&mut self, new_offset: usize) {
        self.new_offset = new_offset;
    }

    pub fn get_length(&self) -> usize {
        let output;

        if self.length < 0 {
            output = (0 - self.length) + 1;
        } else {
            output = self.length;
        }

        output
    }

    pub fn get_offset(&self) -> usize {
        let output;

        if self.length < 0 {
            output =  self.offset + self.length;
        } else {
            output = self.offset;
    }
}


trait Editor {
    pub fn undo(&mut self);
    pub fn replace(&mut self, search_for: Vec<u8>, replace_with: Vec<u8>);
    pub fn set_cursor_offset(&mut self, new_offset: usize);
    pub fn set_cursor_length(&mut self, new_length: usize);
    pub fn make_selection(&mut self, offset: usize, length: usize);
    pub fn copy_to_clipboard(&mut self, bytes_to_copy: Vec<u8>);
    pub fn copy_selection(&mut self);
    pub fn load_file(&mut self, file_path: String);
    pub fn save_file(&mut self);
    pub fn set_file_path(&mut self, new_file_path: String);
    pub fn find_all(&self, pattern: Vec<u8>) -> Vec<usize>;
    pub fn find_after(&self, pattern: Vec<u8>, offset: usize) -> Option(usize);
    pub fn backspace(&mut self);
    pub fn remove_bytes(&mut self, offset: usize, length: usize);
    pub fn insert_bytes(&mut self, new_bytes: Vec<u8>, offset: usize);
    pub fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>);
    pub fn overwrite_bytes(&mut self, new_bytes: Vec<u8>, offset: usize);
    pub fn get_selected(&mut self) -> Vec<u8>;
    pub fn cursor_move(&mut self, new_offset: usize);
    pub fn cursor_next_byte(&mut self);
    pub fn cursor_prev_byte(&mut self);
    pub fn cursor_increase_length(&mut self);
    pub fn cursor_decrease_length(&mut self);
}

trait VisualEditor {
    pub fn cursor_next_line(&mut self);
    pub fn cursor_prev_line(&mut self);
    pub fn cursor_increase_length_by_line(&mut self);
    pub fn cursor_decrease_length_by_line(&mut self);
    pub fn adjust_viewport_offset(&mut self);
}

struct ViewPort {
    offset: usize,
    width: usize,
    height: usize
}

impl ViewPort {
    pub fn get_width(&self) {
        self.width
    }
    pub fn get_height(&self) {
        self.height
    }
    pub fn get_ofset(&self) {
        self.offset
    }
    pub fn set_offset(&mut self, new_offset: usize) {
        self.offset = new_offset;
    }
    pub fn set_width(&mut self, new_width: usize) {
        self.width = new_width;
    }
    pub fn set_height(&mut self, new_height: usize) {
        self.height = new_height;
    }
    pub fn set_size(&mut self, new_width: usize, new_height: usize) {
        self.set_width(new_width);
        self.set_height(new_height);
    }
}

trait UI {
    pub fn set_user_mode(&mut self) {

    }
    pub fn get_user_mode(&mut self) {

    }
}

struct HunkEditor {
    //Editor
    clipboard: Vec<u8>,
    active_content: Vec<u8>,
    active_file_path: String,
    internal_log: Vec<String>,
    cursor: Cursor,
    active_converter: Option<u8>,
    converters: HashMap<u8, Converter>,

    // UI
    mode_user: u8,


    // VisualEditor
    viewport: Viewport,

    // ConsoleEditor
}

impl Contextual for HunkEditor {
}

impl Editor for HunkEditor {
    pub fn undo(&mut self) { }

    pub fn replace(&mut self, search_for: Vec<u8>, replace_with: Vec<u8>) { }

    pub fn set_cursor_offset(&mut self, new_offset: usize) {
        let mut adj_offset = cmp::min(self.active_content.len(), new_offset);
        self.cursor.set_offset(adj_offset);
        self.set_cursor_length(self.cursor.length);
    }

    pub fn set_cursor_length(&mut self, new_length: isize) {
        let mut adj_length;
        if new_length < 0 {
            adj_length = cmp::max(new_length, 0 - new_length);
            self.cursor.set_length(adj_length);
        } else if new_length == 0 {
        } else {
            adj_length = cmp::min(new_length, self.active_content.len() - self.cursor.offset);
            self.cursor.set_length(adj_length);
        }
    }

    pub fn make_selection(&mut self, offset: usize, length: usize) {
        self.set_cursor_offset(offset);
        self.set_cursor_length(length);
    }

    pub fn copy_to_clipboard(&mut self, bytes_to_copy: Vec<u8>) { }

    pub fn copy_selection(&mut self) {
        match self.active_content.get_chunk(self.cursor.get_offset(), self.cursor.get_length()) {
            Some(bytes_to_copy) => {
                // TODO: this won't work
                self.copy_to_clipboard(bytes_to_copy);
            }
            None => ()
        }
    }

    pub fn load_file(&mut self, file_path: String) { }

    pub fn save_file(&mut self) { }

    pub fn set_file_path(&mut self, new_file_path: String) { }

    pub fn find_all(&self, pattern: Vec<u8>) -> Vec<usize> { }

    pub fn find_after(&self, pattern: Vec<u8>, offset: usize) -> Option(usize) { }


    pub fn backspace(&mut self) { }

    pub fn remove_bytes(&mut self, offset: usize, length: usize) { }

    pub fn insert_bytes(&mut self, new_bytes: Vec<u8>, position: usize) -> Result<(), ContentError> {
        let mut output;
        if (position < self.bytes.len()) {
            let mut i: usize = position;
            for new_byte in new_bytes.iter() {
                self.bytes.insert(i, new_byte);
                i += 1
            }
            output = Ok(());
        } else {
            output = Err(ContentError::OutOfRange);
        }

        output
    }

    pub fn overwrite_bytes(&mut self, new_bytes: Vec<u8>, position: usize) -> Result<(), ContentError> {
        let mut output;
        if (position < self.bytes.len()) {
            if position + new_bytes.len() < self.bytes.len() {
                let mut i: usize = position;
                for new_byte in new_bytes.iter() {
                    self.bytes[position] = new_byte;
                    position += 1;
                }
            } else {
                self.bytes.resize(position);
                self.bytes.extend_from_slice(&new_bytes.as_slice());
            }
        } else {
            output = Err(ContentError::OutOfRange);
        }

        output
    }


    pub fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) { }

    pub fn get_selected(&mut self) -> Vec<u8> { }

    pub fn cursor_move(&mut self, new_offset: usize) { }
    pub fn cursor_next_byte(&mut self) { }
    pub fn cursor_prev_byte(&mut self) { }
    pub fn cursor_increase_length(&mut self) { }
    pub fn cursor_decrease_length(&mut self) { }
}

impl VisualEditor for HunkEditor {
    pub fn cursor_next_line(&mut self) { }
    pub fn cursor_prev_line(&mut self) { }
    pub fn cursor_increase_length_by_line(&mut self) { }
    pub fn cursor_decrease_length_by_line(&mut self) { }
    pub fn adjust_viewport_offset(&mut self) { }
}

////////////////////////////////////////////////

fn main() {
    println!("Hello, world!");
}
