use asciibox::RectManager;
use terminal_size::{Width, Height, terminal_size};
use std::collections::HashMap;
use std::cmp::{min, max};
use std::fs::File;
use std::io::{Write, Read};

enum FunctionRef {

}

enum EditorError {
    OutOfRange
}

enum ConverterError {
    InvalidDigit
}

enum ConverterRef {
    HEX,
    BIN,
    OCT
}

trait Converter {
    fn encode(&self, human_readable: Vec<u8>) -> Result<Vec<u8>, ConverterError>;
    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError>;
    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError>;
    fn encode_integer(&self, integer: usize) -> Result<Vec<u8>, ConverterError>;
}

struct HexConverter { }

impl HexConverter {
    fn hex_char_to_dec_int(&self, hex_char: u8) -> Result<u8, ConverterError> {
        // TODO: Make constant
        let hex_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];

        match hex_digits.binary_search(&hex_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(e) => {
                Err(ConverterError::InvalidDigit)
            }
        }
    }
}

impl Converter for HexConverter {
    fn encode(&self, human_readable: Vec<u8>) -> Result<Vec<u8>, ConverterError> {
        let mut output_bytes: Vec<u8> = Vec::new();
        let mut output = Ok(Vec::new());

        for byte in human_readable.iter() {
            match self.encode_integer(*byte as usize) {
                Ok(subbytes) => {
                    for subbyte in subbytes.iter() {
                        output_bytes.push(*subbyte);
                    }
                }
                Err(e) => {
                    output = Err(e);
                    break;
                }
            }
        }

        if output.is_ok() {
            output = Ok(output_bytes);
        }

        output
    }

    fn encode_integer(&self, integer: usize) -> Result<Vec<u8>, ConverterError> {
        let first = integer / 16;
        let second = integer % 16;
        let hex_digits = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];


        if (first > hex_digits.len() || second > hex_digits.len()) {
            Err(ConverterError::InvalidDigit)
        } else {

            Ok(vec![hex_digits[first], hex_digits[second]])
        }
    }

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError> {
        let mut output_bytes: Vec<u8> = Vec::new();
        let mut output = Ok(Vec::new());

        let mut byte_value: u8;
        let mut lode_byte = 0;
        for (i, byte) in bytes.iter().rev().enumerate() {
            match self.hex_char_to_dec_int(*byte) {
                Ok(decimal) => {
                    byte_value = decimal;
                    lode_byte += byte_value * ((16_u32.pow((i % 2) as u32)) as u8);

                    if i % 2 != 0 {
                        output_bytes.push(lode_byte);
                        lode_byte = 0;
                    }
                }
                Err(e) => {
                    output = Err(e);
                    break;
                }
            }
        }

        if output.is_ok() {
            if lode_byte != 0 {
                output_bytes.push(lode_byte);
            }

            output_bytes.reverse();
            output = Ok(output_bytes);
        }

        output
    }

    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError> {
        let mut output_number: usize = 0;
        let mut output = Ok(output_number);

        for byte in byte_string.iter() {
            match self.hex_char_to_dec_int(*byte) {
                Ok(decimal_int) => {
                    output_number *= 16;
                    output_number += decimal_int as usize;
                }
                Err(e) => {
                    output = Err(e);
                    break;
                }
            }
        }

        output
    }
}



struct InputNode {
    next_nodes: HashMap<u8, InputNode>,
    hook: Option<FunctionRef>
}


impl InputNode {
    fn new() -> InputNode {
        InputNode {
            next_nodes: HashMap::new(),
            hook: None
        }
    }

    fn assign_command(&mut self, new_pattern: Vec<u8>, hook: FunctionRef) {
        let mut tmp_pattern = Vec::new();
        for (i, byte) in new_pattern.iter().enumerate() {
            tmp_pattern.push(*byte);
        }

        if tmp_pattern.len() > 0 {
            let next_byte = tmp_pattern.remove(0);

            let mut next_node = self.next_nodes.entry(next_byte).or_insert(InputNode::new());
            next_node.assign_command(tmp_pattern, hook);

        } else {
            self.hook = Some(hook);
        }
    }

    fn input(&mut self, new_input: u8) -> bool {
        match self.next_nodes.get(&new_input) {
            Some(_) => {
                true
            }
            None => {
                false
            }
        }
    }
}

struct Cursor {
    offset: usize,
    length: isize
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor {
            offset: 0,
            length: 1
        }
    }
    fn set_length(&mut self, new_length: isize) {
        self.length = new_length;
    }

    fn set_offset(&mut self, new_offset: usize) {
        self.offset = new_offset;
    }

    fn get_length(&self) -> usize {
        let output;

        if self.length < 0 {
            output = (0 - self.length) + 1;
        } else {
            output = self.length;
        }

        output as usize
    }

    fn get_offset(&self) -> usize {
        let output;

        if self.length < 0 {
            output =  self.offset + (self.length as usize);
        } else {
            output = self.offset;
        }

        output
    }
}


trait Editor {
    fn undo(&mut self);
    fn replace(&mut self, search_for: Vec<u8>, replace_with: Vec<u8>);
    fn cursor_set_offset(&mut self, new_offset: usize);
    fn cursor_set_length(&mut self, new_length: isize);
    fn make_selection(&mut self, offset: usize, length: usize);
    fn copy_to_clipboard(&mut self, bytes_to_copy: Vec<u8>);
    fn copy_selection(&mut self);
    fn load_file(&mut self, file_path: String);
    fn save_file(&mut self);
    fn set_file_path(&mut self, new_file_path: String);
    fn find_all(&self, pattern: Vec<u8>) -> Vec<usize>;
    fn find_after(&self, pattern: Vec<u8>, offset: usize) -> Option<usize>;
    fn remove_bytes(&mut self, offset: usize, length: usize);
    fn insert_bytes(&mut self, new_bytes: Vec<u8>, offset: usize) -> Result<(), EditorError>;
    fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>);
    fn overwrite_bytes(&mut self, new_bytes: Vec<u8>, offset: usize) -> Result<(), EditorError>;
    fn get_selected(&mut self) -> Vec<u8>;
    fn cursor_next_byte(&mut self);
    fn cursor_prev_byte(&mut self);
    fn cursor_increase_length(&mut self);
    fn cursor_decrease_length(&mut self);

    fn get_active_converter(&self) -> Box<dyn Converter>;
}

trait VisualEditor {
    fn cursor_next_line(&mut self);
    fn cursor_prev_line(&mut self);
    fn cursor_increase_length_by_line(&mut self);
    fn cursor_decrease_length_by_line(&mut self);
    fn adjust_viewport_offset(&mut self);
}

struct ViewPort {
    offset: usize,
    width: usize,
    height: usize
}

impl ViewPort {
    pub fn new(width: usize, height: usize) -> ViewPort {
        ViewPort {
            offset: 0,
            width: width,
            height: height
        }
    }
    fn get_width(&self) -> usize {
        self.width
    }
    fn get_height(&self) -> usize {
        self.height
    }
    fn get_offset(&self) -> usize {
        self.offset
    }
    fn set_offset(&mut self, new_offset: usize) {
        self.offset = new_offset;
    }
    fn set_width(&mut self, new_width: usize) {
        self.width = new_width;
    }
    fn set_height(&mut self, new_height: usize) {
        self.height = new_height;
    }
    fn set_size(&mut self, new_width: usize, new_height: usize) {
        self.set_width(new_width);
        self.set_height(new_height);
    }
}

enum UserMode {
    MOVE,
    VISUAL,
    COMMAND,
    SEARCH,
    INSERT,
    OVERWRITE
}

trait UI {
    fn set_user_mode(&mut self, mode: UserMode);
    fn get_user_mode(&mut self) -> UserMode;

    fn assign_mode_command(&mut self, mode: UserMode, command_string: Vec<u8>, hook: FunctionRef);
    fn read_input(&mut self, next_byte: u8);
}

enum Undoable { }

struct HunkEditor {
    //Editor
    clipboard: Vec<u8>,
    active_content: Vec<u8>,
    active_file_path: String,
    internal_log: Vec<String>,
    cursor: Cursor,
    active_converter: ConverterRef,
    undo_stack: Vec<(Undoable, usize)>,
    // UI
    mode_user: UserMode,
    input_managers: HashMap<UserMode, InputNode>,

    // VisualEditor
    viewport: ViewPort,

    // ConsoleEditor
}

impl HunkEditor {
    pub fn new() -> HunkEditor {
        let mut width = 1;
        let mut height = 1;

        match terminal_size() {
            Some((Width(w), Height(h))) => {
                width = w as usize;
                height = h as usize;
            }
            None => ()
        }

        HunkEditor {
            clipboard: Vec::new(),
            active_content: Vec::new(),
            active_file_path: String::from("none"),
            internal_log: Vec::new(),
            cursor: Cursor::new(),
            active_converter: ConverterRef::HEX,
            undo_stack: Vec::new(),
            mode_user: UserMode::MOVE,
            input_managers: HashMap::new(),
            viewport: ViewPort::new(width, height)
        }
    }
}

impl Editor for HunkEditor {
    fn undo(&mut self) { }

    fn get_active_converter(&self) -> Box<dyn Converter> {
        match self.active_converter {
            ConverterRef::HEX => {
                Box::new(HexConverter {})
            }
            //ConverterRef::BIN => {
            //    Box::new(BinaryConverter {})
            //}
            _ => {
                Box::new(HexConverter {})
            }
        }
    }

    fn replace(&mut self, search_for: Vec<u8>, replace_with: Vec<u8>) {
        let mut matches = self.find_all(search_for);
        // replace in reverse order
        matches.reverse();

        for i in matches.iter() {
            for j in 0..replace_with.len() {
                self.active_content.remove(i + j);
            }
            for (j, new_byte) in replace_with.iter().enumerate() {
                self.active_content.insert(*i + j, *new_byte);
            }
        }
    }

    fn make_selection(&mut self, offset: usize, length: usize) {
        self.cursor_set_offset(offset);
        self.cursor_set_length(length as isize);
    }

    fn copy_to_clipboard(&mut self, bytes_to_copy: Vec<u8>) {
        self.clipboard = Vec::new();
        for b in bytes_to_copy.iter() {
            self.clipboard.push(*b);
        }
    }

    fn copy_selection(&mut self) {
        let mut selected_bytes = self.get_selected();
        self.copy_to_clipboard(selected_bytes);
    }

    fn load_file(&mut self, file_path: String) {
        match File::open(file_path) {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents);

                self.active_content = Vec::new();

                for (i, byte) in contents.as_bytes().iter().enumerate() {
                    self.active_content.push(*byte);
                }
            }
            Err(e) => {}
        }
    }

    fn save_file(&mut self) {
        let path = &self.active_file_path;
        match File::create(path) {
            Ok(mut file) => {
                file.write_all(self.active_content.as_slice());
                // TODO: Handle potential file system problems
                //file.sync_all();
            }
            Err(e) => {
            }
        }

    }

    fn set_file_path(&mut self, new_file_path: String) {
        self.active_file_path = new_file_path;
    }

    fn find_all(&self, search_for: Vec<u8>) -> Vec<usize> {
        let mut output: Vec<usize> = Vec::new();

        let mut pivot: usize = 0;
        let mut in_match = false;

        let mut search_length = search_for.len();

        for (i, byte) in self.active_content.iter().enumerate() {
            if search_for[pivot] == *byte {
                in_match = true;
                pivot += 1;
            } else {
                in_match = false;
                pivot = 0;
            }

            if pivot == search_length {
                output.push(i - search_length);
            }

        }

        output
    }

    fn find_after(&self, pattern: Vec<u8>, offset: usize) -> Option<usize> {
        //TODO: This could definitely be sped up.
        let mut matches = self.find_all(pattern);
        let mut output = None;

        if matches.len() > 0 {
            for i in matches.iter() {
                if *i >= offset {
                    output = Some(*i);
                    break;
                }
            }
        }

        output
    }


    fn remove_bytes(&mut self, offset: usize, length: usize) {
        let adj_length = min(self.active_content.len() - offset, length);
        for i in 0..adj_length {
            self.active_content.remove(offset);
        }
    }

    fn insert_bytes(&mut self, new_bytes: Vec<u8>, position: usize) -> Result<(), EditorError> {
        let mut output;
        if (position < self.active_content.len()) {
            let mut i: usize = position;
            for new_byte in new_bytes.iter() {
                self.active_content.insert(i, *new_byte);
                i += 1
            }
            output = Ok(());
        } else {
            output = Err(EditorError::OutOfRange);
        }

        output
    }

    fn overwrite_bytes(&mut self, new_bytes: Vec<u8>, position: usize) -> Result<(), EditorError> {
        let mut output;
        if (position < self.active_content.len()) {
            if position + new_bytes.len() < self.active_content.len() {
                for (i, new_byte) in new_bytes.iter().enumerate() {
                    self.active_content[position + i] = *new_byte;
                }
            } else {
                self.active_content.resize(position, 0);
                self.active_content.extend_from_slice(&new_bytes.as_slice());
            }
            output = Ok(());
        } else {
            output = Err(EditorError::OutOfRange);
        }

        output
    }

    fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) {
        let position = self.cursor.get_offset();
        self.insert_bytes(new_bytes, position);
    }

    fn get_selected(&mut self) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();

        for i in offset .. offset + length {
            output.push(self.active_content[i]);
        }

        output
    }

    fn cursor_next_byte(&mut self) {
        let new_position = self.cursor.get_offset() + 1;
        self.cursor_set_offset(new_position);
    }

    fn cursor_prev_byte(&mut self) {
        let new_position = self.cursor.get_offset() - 1;
        self.cursor_set_offset(new_position);
    }

    fn cursor_increase_length(&mut self) {
        let new_length;
        if self.cursor.length == -1 {
            new_length = 1;
        } else {
            new_length = self.cursor.length + 1;
        }

        self.cursor_set_length(new_length);
    }

    fn cursor_decrease_length(&mut self) {
        let new_length;
        if self.cursor.length == 1 {
            new_length = -1
        } else {
            new_length = self.cursor.length - 1;
        }

        self.cursor_set_length(new_length);
    }

    fn cursor_set_offset(&mut self, new_offset: usize) {
        let mut adj_offset = min(self.active_content.len(), new_offset);
        self.cursor.set_offset(adj_offset);
        self.cursor_set_length(self.cursor.length);
    }

    fn cursor_set_length(&mut self, new_length: isize) {
        let mut adj_length;
        if new_length < 0 {
            adj_length = max(new_length, 0 - new_length);
            self.cursor.set_length(adj_length);
        } else if new_length == 0 {
        } else {
            adj_length = min(new_length as usize, self.active_content.len() - self.cursor.offset) as isize;
            self.cursor.set_length(adj_length);
        }
    }
}

impl VisualEditor for HunkEditor {
    fn cursor_next_line(&mut self) {
        let mut new_offset = self.cursor.offset + self.viewport.get_width();
        self.cursor_set_offset(new_offset);
    }

    fn cursor_prev_line(&mut self) {
        let mut new_offset = self.cursor.offset - self.viewport.get_width();
        self.cursor_set_offset(new_offset);
    }

    fn cursor_increase_length_by_line(&mut self) {
        let mut new_length: isize = self.cursor.length + (self.viewport.get_width() as isize);

        if self.cursor.length < 0 && new_length >= 0 {
            new_length += 1;
        }

        self.cursor_set_length(new_length);
    }

    fn cursor_decrease_length_by_line(&mut self) {
        let mut new_length: isize = self.cursor.length - (self.viewport.get_width() as isize);
        if self.cursor.length > 0 && new_length <= 0 {
            new_length -= 1;
        }

        self.cursor_set_length(new_length);
    }

    fn adjust_viewport_offset(&mut self) {
        let width = self.viewport.get_width();
        let height = self.viewport.get_height();
        let screen_buffer_length = width * height;
        let mut adj_viewport_offset = self.viewport.offset;

        if (self.cursor.length >= 0) {
            let cursor_length = self.cursor.length as usize;
            while self.cursor.offset + cursor_length - adj_viewport_offset > screen_buffer_length {
                adj_viewport_offset += width;
            }
        } else {
            let cursor_length = (0 - self.cursor.length) as usize;
            while self.cursor.offset - cursor_length - adj_viewport_offset > screen_buffer_length {
                adj_viewport_offset += width;
            }

        }

        while adj_viewport_offset > self.cursor.offset {
            adj_viewport_offset = max(adj_viewport_offset - width, 0);
        }

        self.viewport.set_offset(adj_viewport_offset);
    }
}

////////////////////////////////////////////////

fn main() {
    let editor = HunkEditor::new();
}
