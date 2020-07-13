use asciibox::{RectManager, Rect, logg};
use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::{time, thread};
use std::env;
use std::sync::{Mutex, Arc};
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};

#[derive(Clone, Copy)]
enum FunctionRef {
    CURSOR_UP,
    CURSOR_DOWN,
    CURSOR_LEFT,
    CURSOR_RIGHT,
    INSERT,
    OVERWRITE,
    DELETE,
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
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8>;
    fn encode_byte(&self, byte: u8) -> Vec<u8>;

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError>;
    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError>;
    fn encode_integer(&self, integer: usize) -> Vec<u8>;
}

struct HexConverter { }
struct HumanConverter { }
struct BinaryConverter { }

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
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        output_bytes
    }

    fn encode_byte(&self, byte: u8) -> Vec<u8> {
        let hex_digits = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];

        let mut output = Vec::new();

        output.push(hex_digits[(byte / 16) as usize]);
        output.push(hex_digits[(byte % 16) as usize]);

        output
    }

    fn encode_integer(&self, mut integer: usize) -> Vec<u8> {
        let hex_digits = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];
        let mut output = Vec::new();
        let mut tmp_hex_digit;
        let mut passes = (integer as f64).log(16.0).ceil() as usize;
        for i in 0 .. passes {
            tmp_hex_digit = integer % 16;
            output.insert(0, hex_digits[tmp_hex_digit]);
            integer /= 16;
        }

        output
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

impl HumanConverter {
    fn dec_char_to_dec_int(&self, dec_char: u8) -> Result<u8, ConverterError> {
        // TODO: Make constant
        let dec_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57];

        match dec_digits.binary_search(&dec_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(e) => {
                Err(ConverterError::InvalidDigit)
            }
        }
    }
}

impl Converter for HumanConverter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output = Vec::new();
        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output.push(*subbyte);
            }
        }

        output
    }

    fn encode_byte(&self, byte: u8) -> Vec<u8> {
        let mut output = Vec::new();
        match byte {
            10 => {
                output.push(226);
                output.push(134);
                output.push(178);
            }
            0..=31 => {
                output.push(46);
            }
            _ => {
                output.push(byte);
            }
        }

        output
    }

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError> {

        Ok(bytes)
    }

    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError> {
        let mut output_number: usize = 0;
        let mut output = Ok(output_number);

        for byte in byte_string.iter() {
            match self.dec_char_to_dec_int(*byte) {
                Ok(decimal_int) => {
                    output_number *= 10;
                    output_number += decimal_int as usize;
                }
                Err(e) => {
                    output = Err(e);
                    break;
                }
            }
        }

        if output.is_ok() {
            output = Ok(output_number);
        }

        output
    }

    fn encode_integer(&self, mut integer: usize) -> Vec<u8> {
        let digits = vec![48,49,50,51,52,53,54,55,56,57];
        let mut did_first_pass = false;
        let mut output = Vec::new();
        let mut test_byte;
        while integer > 0 || ! did_first_pass {
            test_byte = integer % 10;
            output.push(digits[test_byte]);
            integer /= 10;
            did_first_pass = true;
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

    fn fetch_command(&mut self, input_pattern: Vec<u8>) -> (Option<FunctionRef>, bool) {
        let mut output = (None, false);
        match (&self.hook) {
            Some(hook) => {
                // Found, Clear buffer
                output = (Some(*hook), true);
            }
            None => {
                let mut tmp_pattern = input_pattern.clone();
                if tmp_pattern.len() > 0 {
                    let next_byte = tmp_pattern.remove(0);
                    match self.next_nodes.get_mut(&next_byte) {
                        Some(node) => {
                            output = node.fetch_command(tmp_pattern);
                        }
                        None => {
                            // Dead End, Clear Buffer
                            output = (None, true);
                        }
                    };
                } else {
                    // Nothing Found Yet, keep buffer
                    output = (None, false);
                }
            }
        };

        output
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
    fn get_chunk(&mut self, offset: usize, length: usize) -> Vec<u8>;
    fn cursor_next_byte(&mut self);
    fn cursor_prev_byte(&mut self);
    fn cursor_increase_length(&mut self);
    fn cursor_decrease_length(&mut self);

    fn get_active_converter(&self) -> Box<dyn Converter>;
    fn get_display_ratio(&mut self) -> u8;
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
    MOVE = 0,
    VISUAL = 1,
    COMMAND = 2,
    SEARCH = 3,
    INSERT = 4,
    OVERWRITE = 5
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
    mode_user: u8,

    // VisualEditor
    viewport: ViewPort,

    // InConsole
    rectmanager: RectManager,
    active_row_map: HashMap<usize, bool>,
    flag_kill: bool,
    flag_force_rerow: bool,
    file_loaded: bool,

    flag_refresh_full: bool,
    flag_refresh_display: bool,
    flag_refresh_meta: bool,
    cells_to_refresh: HashSet<(usize, usize)>, // rect ids, rather than coords
    rows_to_refresh: HashSet<(usize, usize)>, // rects ids, rather than row

    is_resizing: bool,

    rect_display_wrapper: usize,
    rects_display: (usize, usize),
    rect_meta: usize,

    row_dict: HashMap<usize, (usize, usize)>,
    cell_dict: HashMap<usize, HashMap<usize, (usize, usize)>>
}

impl HunkEditor {
    pub fn new() -> HunkEditor {
        let mut rectmanager = RectManager::new();
        let (width, height) = rectmanager.get_rect_size(0).ok().unwrap();
        let mut id_display_wrapper = rectmanager.new_rect(Some(0));
        let mut id_display_bits = rectmanager.new_rect(
            Some(id_display_wrapper)
        );
        let mut id_display_human = rectmanager.new_rect(
            Some(id_display_wrapper)
        );
        let mut id_rect_meta = rectmanager.new_rect(Some(0));

        HunkEditor {
            clipboard: Vec::new(),
            active_content: Vec::new(),
            active_file_path: String::from("none"),
            internal_log: Vec::new(),
            cursor: Cursor::new(),
            active_converter: ConverterRef::HEX,
            undo_stack: Vec::new(),
            mode_user: UserMode::MOVE as u8,
            viewport: ViewPort::new(width, height),

            rectmanager: rectmanager,

            active_row_map: HashMap::new(),
            flag_kill: false,
            flag_force_rerow: false,
            file_loaded: false,

            flag_refresh_full: false,
            flag_refresh_display: false,
            flag_refresh_meta: false,
            cells_to_refresh: HashSet::new(),
            rows_to_refresh: HashSet::new(),

            is_resizing: false,

            rect_display_wrapper: id_display_wrapper,
            rects_display: (id_display_bits, id_display_human),
            rect_meta: id_rect_meta,

            row_dict: HashMap::new(),
            cell_dict: HashMap::new()
        }
    }

    pub fn main(&mut self) {
        let function_refs: Arc<Mutex<Vec<FunctionRef>>> = Arc::new(Mutex::new(Vec::new()));


        let mut input_daemon;

        let c = function_refs.clone();
        let mut tmp_char: &[u8];
        input_daemon = thread::spawn(move || {
            let mut inputter = Inputter::new();
            inputter.assign_mode_command(0, vec![106], FunctionRef::CURSOR_DOWN);
            inputter.assign_mode_command(0, vec![107], FunctionRef::CURSOR_UP);



            // TODO: Clean this shit up. for now, this is how we're reading single character input
            let stdin = 0; // couldn't get std::os::unix::io::FromRawFd to work 
                           // on /dev/stdin or /dev/tty
            let termios = Termios::from_fd(stdin).unwrap();
            let mut new_termios = termios.clone();  // make a mutable copy of termios 
                                                    // that we will modify
            new_termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
            tcsetattr(stdin, TCSANOW, &mut new_termios).unwrap();
            let stdout = io::stdout();
            let mut reader = io::stdin();
            let mut buffer;

            stdout.lock().flush().unwrap();

            while true {
                buffer = [0;1];
                reader.read_exact(&mut buffer).unwrap();
                for character in buffer.iter() {
                    match inputter.read_input(*character) {
                        Some(funcref) => {

                            // Just wait until the FunctionRef can be added to the queue
                            while true {
                                match c.try_lock() {
                                    Ok(ref mut mutex) => {
                                        mutex.push(funcref);
                                        break;
                                    }
                                    Err(e) => {}
                                }
                            }

                        }
                        None => {
                        }
                    }
                }
            }
        });


        let fps = 60.0;

        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);
        self.setup_displays();

        let mut _current_func;
        while ! self.flag_kill {
            match function_refs.try_lock() {
                Ok(ref mut mutex) => {
                    if mutex.len() > 0 {
                        _current_func = mutex.remove(0);
                        self.run_cmd_from_functionref(_current_func);
                    }
                }
                Err(e) => {
                }
            }

            self.tick();
            thread::sleep(delay);
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

                self.file_loaded = true;
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
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();

        self.get_chunk(offset, length)
    }

    fn get_chunk(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
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

    fn get_display_ratio(&mut self) -> u8 {
        3
        //let human_converter = HumanConverter {};
        //let human_string_length = human_converter.encode(vec![65]).len();

        //let active_converter = self.get_active_converter();
        //let active_string_length = active_converter.encode(vec![65]).len();

        //(active_string_length / human_string_length) as u8
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

trait UI {
    fn set_user_mode(&mut self, mode: u8);
    fn get_user_mode(&mut self) -> u8;

    fn run_cmd_from_functionref(&mut self, funcref: FunctionRef);
}

impl UI for HunkEditor {
    fn set_user_mode(&mut self, mode: u8) {
        self.mode_user = mode;
    }

    fn get_user_mode(&mut self) -> u8 {
        self.mode_user
    }


    fn run_cmd_from_functionref(&mut self, funcref: FunctionRef) {
        match funcref {
            FunctionRef::CURSOR_UP => {
                self.remove_cursor();
                self.cursor_prev_line();
                self.adjust_viewport_offset();
                self.apply_cursor();
            }
            FunctionRef::CURSOR_DOWN => {
                self.remove_cursor();
                self.cursor_next_line();
                self.adjust_viewport_offset();
                self.apply_cursor();
            }
            FunctionRef::CURSOR_LEFT => {
                self.cursor_prev_byte();
            }
            FunctionRef::CURSOR_RIGHT => {
                self.cursor_next_byte();
            }
            _ => {
                // Unknown
            }
        }
    }
}

struct Inputter {
    input_managers: HashMap<u8, InputNode>,
    input_buffer: Vec<u8>,
    context: u8
}

impl Inputter {
    fn new() -> Inputter {
        Inputter {
            input_managers: HashMap::new(),
            input_buffer: Vec::new(),
            context: 0
        }
    }

    fn read_input(&mut self, input_byte: u8) -> Option<FunctionRef> {
//        let input: Option<u8> = std::io::stdin()
//            .bytes()
//            .next()
//            .and_then(|result| result.ok())
//            .map(|byte| byte as u8);
//
        let mut output = None;

        self.input_buffer.push(input_byte);

        let input_buffer = self.input_buffer.clone();
        let mut clear_buffer = false;

        match self.input_managers.get_mut(&self.context) {
            Some(root_node) => {
                let (cmd, completed_path) = root_node.fetch_command(input_buffer);
                match cmd {
                    Some(funcref) => {
                        output = Some(funcref);
                    }
                    None => ()
                }
                clear_buffer = completed_path;
            }
            None => ()
        }

        if (clear_buffer) {
            self.input_buffer.drain(..);
        }

        output
    }

    fn assign_mode_command(&mut self, mode: u8, command_string: Vec<u8>, hook: FunctionRef) {
        let mut mode_node = self.input_managers.entry(mode).or_insert(InputNode::new());
        mode_node.assign_command(command_string, hook);
    }
}

trait InConsole {
    fn tick(&mut self);

    fn check_resize(&mut self);
    fn setup_displays(&mut self);
    fn apply_cursor(&mut self);
    fn remove_cursor(&mut self);

    fn remap_active_rows(&mut self);

    fn set_row_characters(&mut self, offset: usize);
    fn autoset_viewport_size(&mut self);

    fn _set_offset_display(&mut self);
    fn arrange_displays(&mut self);
    fn display_user_message(&mut self);
}

impl InConsole for HunkEditor {

    fn tick(&mut self) {
        if ! self.file_loaded {
        } else {
            self.check_resize();
            let mut do_draw = self.flag_refresh_full
                || self.flag_refresh_display
                || self.flag_refresh_meta
                || self.cells_to_refresh.len() > 0
                || self.rows_to_refresh.len() > 0;

            if self.flag_refresh_full {
                self.rectmanager.queue_draw(0);
                self.flag_refresh_full = false;
                self.flag_refresh_display = false;
                self.flag_refresh_meta = false;
                self.cells_to_refresh.drain();
                self.rows_to_refresh.drain();
            }

            if self.flag_refresh_display {
                self.rectmanager.queue_draw(self.rect_display_wrapper);
                self.flag_refresh_display = false;
                self.cells_to_refresh.drain();
                self.rows_to_refresh.drain();
            }

            for (_bits_id, _human_id) in self.cells_to_refresh.iter() {
                self.rectmanager.queue_draw(*_bits_id);
                self.rectmanager.queue_draw(*_human_id);
            }

            for (_bits_id, _human_id) in self.rows_to_refresh.iter() {
                self.rectmanager.queue_draw(*_bits_id);
                self.rectmanager.queue_draw(*_human_id);
            }

            self.cells_to_refresh.drain();
            self.rows_to_refresh.drain();

            if self.flag_refresh_meta {
                self.flag_refresh_meta = false;
                self.rectmanager.queue_draw(self.rect_meta);
            }

            if do_draw {
                self.rectmanager.draw_queued();
            }
        }
    }

    fn autoset_viewport_size(&mut self) {
        let full_height = self.rectmanager.get_height();
        let full_width = self.rectmanager.get_width();
        let meta_height = self.rectmanager.get_rect_size(self.rect_meta).ok().unwrap().1;

        let display_ratio = self.get_display_ratio() as f64;
        let r: f64 = (1f64 / display_ratio);
        let a: f64 = (1f64 - ( 1f64 / (r + 1f64)));
        let base_width = (full_width as f64) * a;

        self.viewport.set_size(
            base_width as usize,
            full_height - meta_height
        );

        self.active_row_map.drain();
        for i in 0 .. self.viewport.height {
            self.active_row_map.insert(i, false);
        }
    }

    fn setup_displays(&mut self) {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();

        self.autoset_viewport_size();
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();

        self.rectmanager.resize(self.rect_meta, full_width, 1);
        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            full_height - 1
        );

        let (bits_display, human_display) = self.rects_display;
        self.rectmanager.empty(bits_display);
        self.rectmanager.empty(human_display);

        self.arrange_displays();

        self.cell_dict.drain();
        self.row_dict.drain();

        let display_ratio = self.get_display_ratio() as usize;
        let mut width_bits;
        if display_ratio != 1 {
            width_bits = max(1, display_ratio - 1);
        } else {
            width_bits = display_ratio;
        }

        let viewport_height = self.viewport.get_height();
        let mut _bits_row_id;
        let mut _bits_cell_id;
        let mut _human_row_id;
        let mut _human_cell_id;
        let mut _cells_hashmap;
        for y in 0..viewport_height {
            self.active_row_map.entry(y)
                .and_modify(|e| *e = false)
                .or_insert(false);

            _bits_row_id = self.rectmanager.new_rect(
                Some(bits_display)
            );

            self.rectmanager.resize(
                _bits_row_id,
                (viewport_width * display_ratio) - 1,
                1
            );
            self.rectmanager.set_position(_bits_row_id, 0, y as isize);


            _human_row_id = self.rectmanager.new_rect(
                Some(human_display)
            );
            self.rectmanager.resize(
                _human_row_id,
                viewport_width,
                1
            );
            self.rectmanager.set_position(
                _human_row_id,
                0,
                y as isize
            );

            self.row_dict.entry(y)
                .and_modify(|e| *e = (_bits_row_id, _human_row_id))
                .or_insert((_bits_row_id, _human_row_id));

            _cells_hashmap = self.cell_dict.entry(y).or_insert(HashMap::new());

            for x in 0 .. viewport_width {
                _bits_cell_id = self.rectmanager.new_rect(
                    Some(_bits_row_id)
                );
                self.rectmanager.resize(
                    _bits_cell_id,
                    width_bits,
                    1
                );

                self.rectmanager.set_position(
                    _bits_cell_id,
                    (x * display_ratio) as isize,
                    0
                );

                _human_cell_id = self.rectmanager.new_rect(
                    Some(_human_row_id)
                );


                self.rectmanager.set_position(
                    _human_cell_id,
                    x as isize,
                    0
                );
                self.rectmanager.resize(_human_cell_id, 1, 1);

                _cells_hashmap.entry(x as usize)
                    .and_modify(|e| *e = (_bits_cell_id, _human_cell_id))
                    .or_insert((_bits_cell_id, _human_cell_id));
            }
        }

        if self.file_loaded {
            self.flag_force_rerow = true;
            self.remap_active_rows();
            self.is_resizing = false;
        }
        self.apply_cursor();

        self.flag_refresh_full = true;
    }

    fn check_resize(&mut self) {
        if self.rectmanager.auto_resize() {
            self.is_resizing = true;
            // Viewport offset needs to be set to zero to ensure each line has the correct width
            self.viewport.set_offset(0);
            self.setup_displays();
            self.flag_force_rerow = true;
            self.remap_active_rows();
            self.is_resizing = false;
        }
    }

    fn arrange_displays(&mut self) {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        let mut meta_height = 1;

        self.rectmanager.set_bg_color(self.rect_meta, 1);
        self.rectmanager.set_position(
            self.rect_meta,
            0,
            (full_height - meta_height) as isize
        );


        let mut display_height = full_height - meta_height;
        self.rectmanager.clear(self.rect_display_wrapper);

        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            display_height
        );

        self.rectmanager.set_position(
            self.rect_display_wrapper,
            0,
            0
        );

        let display_ratio = self.get_display_ratio();
        let (bits_id, human_id) = self.rects_display;

        let bits_display_width = self.viewport.get_width() * display_ratio as usize;

        self.rectmanager.resize(bits_id, bits_display_width, display_height);
        self.rectmanager.set_position(bits_id, 0, 0);

        // TODO: Fill in a separator

        let human_display_width = self.viewport.get_width();
        let human_display_x = (full_width - human_display_width) as isize;

        self.rectmanager.resize(human_id, human_display_width, display_height);
        self.rectmanager.set_position(human_id, human_display_x, 0);

        self.flag_refresh_display = true;
    }

    fn remap_active_rows(&mut self) {
        let width = self.viewport.get_width();
        let height = self.viewport.get_height();
        let initial_y = (self.viewport.get_offset() / width) as isize;

        self.adjust_viewport_offset();
        let new_y = (self.viewport.get_offset() / width) as isize;

        let diff: usize;
        if (new_y > initial_y) {
            diff = (new_y - initial_y) as usize;
        } else {
            diff = (initial_y - new_y) as usize;
        }

        if diff > 0 || self.flag_force_rerow {
            if diff < height && ! self.flag_force_rerow {
                // Don't rerender rendered rows. just shuffle them around
                {
                    let (bits, human) = self.rects_display;
                    self.rectmanager.shift_contents(
                        human,
                        0,
                        initial_y - new_y
                    );
                    self.rectmanager.shift_contents(
                        bits,
                        0,
                        initial_y - new_y
                    );
                }

                let mut new_rows_map = HashMap::new();
                let mut new_cells_map = HashMap::new();
                let mut new_active_map = HashMap::new();
                if new_y < initial_y {
                    // Reassign the display_dicts to correspond to correct rows
                    let mut from_y;
                    for y in 0 .. height {
                        from_y = (y - diff) % height;
                        match self.row_dict.get(&from_y) {
                            Some((bits, human)) => {
                                new_rows_map.entry(from_y)
                                    .and_modify(|e| { *e = (*bits, *human)})
                                    .or_insert((*bits, *human));
                            }
                            None => ()
                        }

                        match self.cell_dict.get(&from_y) {
                            Some(cellhash) => {
                                new_cells_map.entry(from_y)
                                    .and_modify(|e| { *e = cellhash.clone()})
                                    .or_insert(cellhash.clone());
                            }
                            None => ()
                        }
                        if y >= height - diff {
                            match new_rows_map.get(&y) {
                                Some((bits, human)) => {
                                    self.rectmanager.set_position(*human, 0, y as isize);
                                    self.rectmanager.set_position(*bits, 0, y as isize);
                                }
                                None => ()
                            }
                            new_active_map.insert(y, false);
                        } else {
                            match self.active_row_map.get(&from_y) {
                                Some(needs_refresh) => {
                                    new_active_map.insert(y, *needs_refresh);
                                }
                                None => ()
                            }
                        }
                    }
                }
                self.active_row_map = new_active_map;
                for (y, (bits, human)) in new_rows_map.iter() {
                    self.row_dict.entry(*y)
                        .and_modify(|e| {*e = (*bits, *human)})
                        .or_insert((*bits, *human));
                }
                for (y, cells) in new_cells_map.iter() {
                    self.cell_dict.entry(*y)
                        .and_modify(|e| {*e = cells.clone()})
                        .or_insert(cells.clone());
                }
            } else {
                for y in 0 .. height {
                    self.active_row_map.entry(y)
                        .and_modify(|e| {*e = false})
                        .or_insert(false);
                }
            }
        }

        let active_rows = self.active_row_map.clone();
        for (y, is_rendered) in active_rows.iter() {
            if ! is_rendered {
                self.set_row_characters(*y + (new_y as usize));
            }
        }

        self.flag_force_rerow = false;
        //TODO
        //self.set_offset_display();
        self.flag_refresh_display = true;
    }

    fn set_row_characters(&mut self, absolute_y: usize) {
        let viewport = &self.viewport;
        let active_converter = self.get_active_converter();
        let human_converter = HumanConverter {};
        let width = viewport.get_width();
        let offset = width * absolute_y;

        let chunk = self.get_chunk(offset, width);
        let relative_y = absolute_y - (self.viewport.get_offset() / width);
        match self.cell_dict.get_mut(&relative_y) {
            Some(mut cellhash) => {
                for (x, (rect_id_bits, rect_id_human)) in cellhash.iter_mut() {
                    self.rectmanager.clear(*rect_id_human);
                    self.rectmanager.clear(*rect_id_bits);
                }

                let mut tmp_bits = vec![65, 65];
                let mut tmp_bits_str;
                let mut tmp_human;
                let mut tmp_human_str;
                for (x, byte) in chunk.iter().enumerate() {
                    tmp_bits = active_converter.encode_byte(*byte);
                    tmp_human = human_converter.encode_byte(*byte);
                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            tmp_bits_str = std::str::from_utf8(tmp_bits.as_slice()).unwrap();
                            tmp_human_str = std::str::from_utf8(tmp_human.as_slice()).unwrap();
                            self.rectmanager.set_string(*human, 0, 0, tmp_human_str);
                            self.rectmanager.set_string(*bits, 0, 0, tmp_bits_str);
                        }
                        None => {
                        }
                    }
                }
            }
            None => {
            }
        }

        match self.row_dict.get(&relative_y) {
            Some((_bits, _human)) => {
                self.rows_to_refresh.insert((*_bits, *_human));
            }
            None => ()
        }


        self.active_row_map.entry(relative_y)
            .and_modify(|e| {*e = true})
            .or_insert(true);
    }

    fn _set_offset_display(&mut self) {
        let mut digit_count = 0;
        if self.active_content.len() > 0 {
            digit_count = (self.active_content.len() as f64).log10().ceil() as usize;
        }
        let offset_display = format!("Offset: {} / {}", self.cursor.get_offset(), self.active_content.len());

        self.rectmanager.clear(self.rect_meta);
        // TODO: Right-align
        let meta_width = self.rectmanager.get_rect_width(self.rect_meta);
        let x = meta_width - offset_display.len();
        self.rectmanager.set_string(self.rect_meta, 0, 0, &offset_display);

        self.flag_refresh_meta = true;
    }

    fn display_user_message(&mut self) {

    }

    fn apply_cursor(&mut self) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let cursor_offset = self.cursor.get_offset() - self.viewport.get_offset();
        let cursor_length = self.cursor.get_length();

        let mut y;
        let mut x;
        for i in cursor_offset .. cursor_offset + cursor_length {
            y = i / viewport_width;
            if y < viewport_height {
                match self.cell_dict.get(&y) {
                    Some(cellhash) => {
                        x = i % viewport_width;
                        match cellhash.get(&x) {
                            Some((bits, human)) => {
                                self.rectmanager.set_invert_flag(*bits);
                                self.rectmanager.set_invert_flag(*human);
                                self.cells_to_refresh.insert((*bits, *human));
                            }
                            None => ()
                        }
                    }
                    None => ()
                }
            }
        }
    }

    fn remove_cursor(&mut self) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let cursor_offset = self.cursor.get_offset() - self.viewport.get_offset();
        let cursor_length = self.cursor.get_length();

        let mut y;
        let mut x;
        for i in cursor_offset .. cursor_offset + cursor_length {
            y = i / viewport_width;
            if y < viewport_height {
                match self.cell_dict.get(&y) {
                    Some(cellhash) => {
                        x = i % viewport_width;
                        match cellhash.get(&x) {
                            Some((bits, human)) => {
                                self.rectmanager.unset_invert_flag(*bits);
                                self.rectmanager.unset_invert_flag(*human);
                                self.cells_to_refresh.insert((*bits, *human));
                            }
                            None => ()
                        }
                    }
                    None => ()
                }
            }
        }
    }
}


////////////////////////////////////////////////

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut editor = HunkEditor::new();
    editor.load_file(args.get(1).unwrap().to_string());
    editor.main();
}
