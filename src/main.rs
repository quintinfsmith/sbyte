use asciibox::{RectManager, Rect};
use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fs::File;
use std::io::{Write, Read};
use std::{time, thread};

enum FunctionRef { }

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
    MOVE,
    VISUAL,
    COMMAND,
    SEARCH,
    INSERT,
    OVERWRITE
}
impl std::cmp::PartialEq for UserMode {}
impl std::cmp::Eq for UserMode { }
impl std::hash::Hash for UserMode { }

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
    pub fn new(width:usize, height: usize) -> HunkEditor {
        let mut rectmanager = RectManager::new();
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
            mode_user: UserMode::MOVE,
            input_managers: HashMap::new(),
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
        let human_string_length;
        match HumanConverter::encode(vec![65]) {
            Ok(_bytes) => {
                human_string_length = _bytes.len();
            }
            Err(e) => {
                // TODO
                human_string_length = 1;
            }
        }

        let active_converter = self.get_active_converter();
        let active_string_length;
        match active_converter.encode(vec![65]) {
            Ok(_bytes) => {
                active_string_length =  _bytes.len();
            }
            Err(e) => {
                // TODO
                active_string_length = 1;
            }
        }
        (active_string_length / human_string_length) as u8
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
    fn set_user_mode(&mut self, mode: UserMode);
    fn get_user_mode(&mut self) -> UserMode;

    fn assign_mode_command(&mut self, mode: UserMode, command_string: Vec<u8>, hook: FunctionRef);
    fn read_input(&mut self, next_byte: u8);
}
impl UI for HunkEditor {
    fn set_user_mode(&mut self, mode: UserMode) {
        self.mode_user = mode;
    }

    fn get_user_mode(&mut self) -> UserMode {
        self.mode_user
    }

    fn assign_mode_command(&mut self, mode: UserMode, command_string: Vec<u8>, hook: FunctionRef) {
       let mut mode_node = self.input_managers.entry(mode).or_insert(InputNode::new());
        mode_node.assign_command(command_string, hook);
    }

    fn read_input(&mut self, next_byte: u8) {

    }
}

trait InConsole {
    fn run_display(&mut self, fps: f64);
    fn tick(&mut self);

    fn check_resize(&mut self);
    fn setup_displays(&mut self);
    fn apply_cursor(&mut self);
    fn remove_cursor(&mut self);

    fn remap_active_rows(&mut self);

    fn set_row_characters(&mut self, offset: usize);
    fn autoset_viewport_size(&mut self);
}

impl InConsole for HunkEditor {
    fn run_display(&mut self, fps: f64) {
        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);
        while ! self.flag_kill {
            self.tick();
            thread::sleep(delay);
        }
    }

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
                self.rectmanager.queue_draw(_bits_id);
                self.rectmanager.queue_draw(_human_id);
            }

            for (_bits_id, _human_id) in self.rows_to_refresh.iter() {
                self.rectmanager.queue_draw(_bits_id);
                self.rectmanager.queue_draw(_human_id);
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
        let meta_height = self.rectmanager.get_rect_height(self.rect_meta);

        let display_ratio = self.get_display_ratio();
        let r = (1 / display_ratio);
        let a = (1 - ( 1 / r + 1));
        let base_width = full_width * a;
        self.viewport.set_size(
            base_width,
            full_height - 1
        )

        self.active_row_map.drain();
        for i in 0 .. self.viewport.height {
            self.active_row_map.insert(i, false);
        }
    }

    fn setup_displays(&mut self) {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        self.autoset_viewport_size();
        let viewport = self.viewport;
        let viewport_width = viewport.get_width();
        let viewport_height = viewport.get_height();

        self.rectmanager.resize(self.rect_meta, full_width, 1);
        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            full_height - 1
        );

        let (bits_display, human_display) = self.rects_display;
        self.rectmanager.empty(bits_display);
        self.rectmanager.empty(human_display);

        self.cell_dict.drain();
        self.row_dict.drain();

        let display_ratio = self.get_display_ratio() as usize;
        let mut width_bits;
        if display_ratio != 1 {
            width_bits = max(1, display_ratio - 1);
        } else {
            width_bits = display_ratio;
        }

        let viewport_height = viewport.get_height();
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
            self.rectmanager.set_position(_bits_row_id, 0, y);


            _human_row_id = self.rectmanager.new_rect(
                Some(human_display)
            );
            self.rectmanager.resize(
                _human_row_id,
                viewport_width
                1
            );
            self.rectmanager.set_position(_human_row_id, 0, y);

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
                    x * display_ratio,
                    0
                );

                _human_cell_id = self.rectmanager.new_rect(
                    Some(_human_row_id)
                );

                self.rectmanager.set_position(_human_cell_id, x, 0);
                self.rectmanager.resize(_human_cell_id, 1, 1);

                _cells_hashmap.entry(x as usize)
                    .and_modify(|e| *e = (_bits_cell_id, _human_cell_id))
                    .or_insert((_bits_cell_id, _human_cell_id));
            }
        }

        self.flag_refresh_meta = true;
    }

    fn check_resize(&mut self) {
        let mut rectmanager = self.rectmanager;
        if rectmanager.check_resize() {
            self.is_resizing = true;
            // Viewport offset needs to be set to zero to ensure each line has the correct width
            self.viewport.set_offset(0);
            self.setup_displays();
            self.flag_force_rerow = true;
            self.remap_active_rows();
        }
    }

    fn arrange_displays(&mut self) {
        let viewport = self.viewport;
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        let mut meta_height = 0;
        match self.rectmanager.get_rect_mut(self.rect_meta) {
            Some(rect_meta) => {
                meta_height = rect_meta.get_height()
                self.rect_meta.move(0, full_height - meta_height)
            }
            None => ()
        }

        let mut display_height = full_height - meta_height;
        self.rectmanager.get_rect_mut(self.rect_display_wrapper) {
            Some(display_wrapper) => {
                display_wrapper.clear();
                display_wrapper.resize(
                    full_width,
                    display_height
                );
                display_wrapper.move(0, 0);
            },
            // TODO: Throw Error
            None => ()
        }

        let display_ratio = self.get_display_ratio();
        let (human_id, bits_id) = self.rects_display;
        let bits_display_width = viewport.get_width() * display_ratio;
        match self.rectmanager.get_mut(bits_id) {
            Some(rect_bits) => {
                rect_bits.resize(
                    bits_display_width,
                    display_height
                );
                rect_bits.move(0, 0);
            }
            None => ()
        }
        // TODO: Fill in a separator

        let human_display_width = viewport.get_width();
        let human_display_x = full_width - human_display_width;
        match self.rectmanager.get_mut(human_id) {
            Some(rect_human) => {
                rect_human.resize(
                    human_display_width,
                    display_height
                );
                rect_human.move(human_display_x, 0);
            }
            None => ()
        }

        self.flags_refresh_display = true;
    }

    fn remap_active_rows(&mut self) {
        let viewport = self.viewport;
        let width = viewport.get_width();
        let height = viewport.get_height();
        let initial_y = viewport.get_offset() / width;

        self.adjust_viewport_offset();

        let new_y = viewport.get_offset() / width;

        let diff;
        if (new_y > initial_y) {
            diff = new_y - initial_y;
        } else {
            diff = initial_y - new_y;
        }

        if diff > 0 || self.flag_force_rerow {
            if diff < height && ! self.flag_force_rerow {
                // Don't rerender rendered rows. just shuffle them around
                {
                    let (bits, human) = self.rects_display;
                    self.rectmanager.shift_contents(
                        human,
                        initial_y - new_y
                    );
                    self.rectmanager.shift_contents(
                        bits,
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
                                    .and_modify(|e| { *e = *cellhash})
                                    .or_insert(*cellhash);
                            }
                            None => ()
                        }
                        if y >= height - diff {
                            match new_rows_map.get(&y) {
                                Some((bits, human)) => {
                                    self.rectmanager.set_position(human, 0, y);
                                    self.rectmanager.set_position(bits, 0, y);
                                }
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
                        .and_modify(|e| {*e = *cells})
                        .or_insert(*cells);
                }
            } else {
                for y in 0 .. height {
                    self.active_row_map.entry(y)
                        .and_modify(|e| {*e = false})
                        .or_insert(false);
                }
            }
        }
        let iterator = self.active_row_map.iter();
        for (y, is_rendered) in iterator {
            if ! is_rendered {
                self.set_row_characters(*y + new_y);
            }
        }

        self.flag_force_rerow = false;
        //TODO
        //self.set_offset_display();
        self.flag_refresh_display = true;
    }

    fn set_row_characters(&mut self, absolute_y: usize) {
        let viewport = self.viewport;
        let width = viewport.get_width();
        let offset = width * absolute_y;

        let chunk = self.get_chunk(offset, width);
        let relative_y = absolute_y - (viewport.get_offset() / width);
        match self.cell_dict.get_mut(&relative_y) {
            Some(mut cellhash) => {
                for (x, (rect_id_bits, rect_id_human)) in cellhash.iter_mut() {
                    self.rectmanager.clear(rect_id_human);
                    self.rectmanager.clear(rect_id_bits);
                }

                let mut tmp_human;
                let mut tmp_bits;
                for (x, byte) in chunk.iter().enumerate() {
                    //TODO: HumanConverter
                    tmp_bits = self.get_active_converter().encode_integer(*byte as usize);
                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            self.rectmanager.set_string(human, 0, 0, tmp_human);
                            self.rectmanager.set_string(bits, 0, 0, tmp_bits);
                        }
                        None => ()
                    }
                }
            }
            None => ()
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

    }

    fn display_user_message(&mut self) {

    }

    fn apply_cursor(&mut self) {
    }
    fn remove_cursor(&mut self) {
    }

}


////////////////////////////////////////////////

fn main() {
    let editor = HunkEditor::new(10, 10);
}
