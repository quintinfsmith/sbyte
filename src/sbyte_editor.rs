use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::error::Error;
use std::{time, thread};
use std::sync::{Mutex, Arc};
use std::fmt;

use wrecked::{RectManager, RectColor, RectError};

// Editor trait
pub mod editor;
// VisualEditor trait
pub mod visual_editor;
// Commandable trait;
pub mod commandable;
// InConsole trait
pub mod inconsole;
// CommandLine struct
pub mod command_line;

//Structured data
pub mod structured;

pub mod command_interface;

use editor::{Editor, EditorError};
use editor::editor_cursor::Cursor;
use editor::converter::{HumanConverter, BinaryConverter, HexConverter, Converter, ConverterRef, ConverterError, DecConverter};
use visual_editor::*;
use visual_editor::viewport::ViewPort;
use commandable::Commandable;
use commandable::inputter::Inputter;
use command_line::CommandLine;
use inconsole::*;
use structured::*;
use command_interface::CommandInterface;


pub struct InputterEditorInterface {
    function_queue: Vec<(String, Vec<u8>)>,

    new_context: Option<String>,
    new_input_sequences: Vec<(String, String, String)>,

    flag_kill: bool
}

impl InputterEditorInterface {
    pub fn new() -> InputterEditorInterface {
        InputterEditorInterface {
            function_queue: Vec::new(),

            new_context: None,
            new_input_sequences: Vec::new(),

            flag_kill: false
        }
    }
}

#[derive(Debug)]
pub enum SbyteError {
    PathNotSet,
    SetupFailed(RectError),
    RemapFailed(RectError),
    RowSetFailed(RectError),
    ApplyCursorFailed(RectError),
    DrawFailed(RectError)
}
impl fmt::Display for SbyteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for SbyteError {}


pub struct SbyteEditor {
    surpress_tick: bool, // Used to prevent visual feedback

    // Flags for tick() to know when to arrange/edit rects
    display_flags: HashMap<Flag, (usize, bool)>,
    display_flag_timeouts: HashMap<Flag, usize>,

    user_msg: Option<String>,
    user_error_msg: Option<String>,


    //Editor
    clipboard: Vec<u8>,
    active_content: Vec<u8>,
    active_file_path: Option<String>,
    cursor: Cursor,
    active_converter: ConverterRef,
    undo_stack: Vec<(usize, usize, Vec<u8>)>, // Position, bytes to remove, bytes to insert
    redo_stack: Vec<(usize, usize, Vec<u8>)>, // Position, bytes to remove, bytes to insert
    has_unsaved_changes: bool,


    // Commandable
    commandline: CommandLine,
    line_commands: HashMap<String, String>,
    register: Option<usize>,
    flag_input_context: Option<String>,
    new_input_sequences: Vec<(String, String, String)>,

    // VisualEditor
    viewport: ViewPort,

    // InConsole
    rectmanager: RectManager,
    active_row_map: HashMap<usize, bool>,
    flag_kill: bool,
    flag_force_rerow: bool,
    locked_viewport_width: Option<usize>,

    cells_to_refresh: HashSet<(usize, usize)>, // rect ids, rather than coords
    rows_to_refresh: Vec<usize>, // absolute row numbers
    active_cursor_cells: HashSet<(usize, usize)>, //rect ids of cells highlighted by cursor

    is_resizing: bool,

    rect_display_wrapper: usize,
    rects_display: (usize, usize),
    rect_meta: usize,

    row_dict: HashMap<usize, (usize, usize)>,
    cell_dict: HashMap<usize, HashMap<usize, (usize, usize)>>,

    search_history: Vec<Vec<u8>>,

    structure_id_gen: u64,
    structures: HashMap<u64, Box<dyn StructuredDataHandler>>,
    structure_spans: HashMap<u64, (usize, usize)>,
    structure_map: HashMap<usize, HashSet<u64>>,
    structure_validity: HashMap<u64, bool>,
}

impl SbyteEditor {
    pub fn new() -> SbyteEditor {
        let mut rectmanager = RectManager::new();
        let (width, height) = rectmanager.get_rect_size(wrecked::TOP).unwrap();
        let id_display_wrapper = rectmanager.new_rect(wrecked::TOP).ok().unwrap();
        let id_display_bits = rectmanager.new_rect(id_display_wrapper).ok().unwrap();
        let id_display_human = rectmanager.new_rect(id_display_wrapper).ok().unwrap();

        let id_rect_meta = rectmanager.new_rect(wrecked::TOP).ok().unwrap();

        let mut flag_timeouts = HashMap::new();
        flag_timeouts.insert(Flag::CURSOR_MOVED, 1);
        flag_timeouts.insert(Flag::SETUP_DISPLAYS, 0);
        flag_timeouts.insert(Flag::REMAP_ACTIVE_ROWS, 2);
        flag_timeouts.insert(Flag::UPDATE_OFFSET, 0);

        for i in 0 .. 60 {
            flag_timeouts.insert(Flag::UPDATE_ROW(i), 5);
        }

        let mut output = SbyteEditor {
            surpress_tick: false,

            display_flags: HashMap::new(),
            display_flag_timeouts: flag_timeouts,

            user_msg: None,
            user_error_msg: None,

            clipboard: Vec::new(),
            active_content: Vec::new(),
            active_file_path: None,
            cursor: Cursor::new(),
            active_converter: ConverterRef::HEX,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            has_unsaved_changes: false,
            register: None,
            flag_input_context: None,
            new_input_sequences: Vec::new(),

            viewport: ViewPort::new(width, height),

            line_commands: HashMap::new(),
            commandline: CommandLine::new(),
            rectmanager,

            active_row_map: HashMap::new(),
            flag_kill: false,
            flag_force_rerow: false,
            locked_viewport_width: None,

            cells_to_refresh: HashSet::new(),
            rows_to_refresh: Vec::new(),
            active_cursor_cells: HashSet::new(),

            is_resizing: false,

            rect_display_wrapper: id_display_wrapper,
            rects_display: (id_display_bits, id_display_human),
            rect_meta: id_rect_meta,

            row_dict: HashMap::new(),
            cell_dict: HashMap::new(),

            search_history: Vec::new(),

            structure_id_gen: 0,
            structures: HashMap::new(),
            structure_spans: HashMap::new(),
            structure_validity: HashMap::new(),
            structure_map: HashMap::new()
        };

        output.assign_line_command("q", "QUIT");
        output.assign_line_command("w", "SAVE");
        output.assign_line_command("wq", "SAVEQUIT");
        output.assign_line_command("find", "JUMP_TO_NEXT");
        output.assign_line_command("insert", "INSERT_STRING");
        output.assign_line_command("overwrite", "OVERWRITE");
        output.assign_line_command("setcmd", "ASSIGN_INPUT");
        output.assign_line_command("lw", "SET_WIDTH");
        output.assign_line_command("reg", "SET_REGISTER");

        output.raise_flag(Flag::SETUP_DISPLAYS);
        output.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        output
    }

    pub fn load_config(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        match File::open(file_path) {
            Ok(mut file) => {
                let file_length = match file.metadata() {
                    Ok(metadata) => {
                        metadata.len()
                    }
                    Err(_e) => {
                        0
                    }
                };

                let mut buffer: Vec<u8> = vec![0; file_length as usize];
                file.read(&mut buffer)?;

                let working_cmds: Vec<&str> = std::str::from_utf8(buffer.as_slice()).unwrap().split("\n").collect();
                self.surpress_tick = true;
                for cmd in working_cmds.iter() {
                    self.try_command(cmd)?;
                }
                self.surpress_tick = false;
            }
            Err(_e) => { }
        }

        Ok(())
    }

    pub fn main(&mut self) -> Result<(), Box<dyn Error>> {
        let input_interface: Arc<Mutex<InputterEditorInterface>> = Arc::new(Mutex::new(InputterEditorInterface::new()));

        let signal_mutex = input_interface.clone();
        let mut kill_daemon = ctrlc::set_handler(move || {
            let mut ok = false;
            while !ok {
                match signal_mutex.try_lock() {
                    Ok(ref mut mutex) => {
                        mutex.flag_kill = true;
                        ok = true;
                    }
                    Err(_e) => ()
                }
            }
        }).expect("Error setting Ctrl-C handler");

        let c = input_interface.clone();
        let mut _input_daemon = thread::spawn(move || {
            let mut inputter = Inputter::new();

            let mode_default = "DEFAULT";
            let mode_insert = "INSERT";
            let mode_overwrite = "OVERWRITE";
            let mode_cmd = "CMD";

            inputter.assign_context_switch("MODE_SET_INSERT", mode_insert);
            inputter.assign_context_switch("MODE_SET_OVERWRITE", mode_overwrite);
            inputter.assign_context_switch("MODE_SET_APPEND", mode_insert);
            inputter.assign_context_switch("MODE_SET_DEFAULT", mode_default);
            inputter.assign_context_switch("MODE_SET_CMD", mode_cmd);
            inputter.assign_context_switch("MODE_SET_SEARCH", mode_cmd);
            inputter.assign_context_switch("MODE_SET_INSERT_SPECIAL", mode_cmd);
            inputter.assign_context_switch("MODE_SET_OVERWRITE_SPECIAL", mode_cmd);
            inputter.assign_context_switch("RUN_CUSTOM_COMMAND", mode_default);

            for i in 0 .. 10 {
                inputter.assign_mode_command(mode_default, std::str::from_utf8(&[i + 48]).unwrap().to_string(), "APPEND_TO_REGISTER");
            }

            inputter.assign_mode_command(mode_insert, std::str::from_utf8(&[27]).unwrap().to_string(), "MODE_SET_DEFAULT");
            inputter.assign_mode_command(mode_overwrite, std::str::from_utf8(&[27]).unwrap().to_string(), "MODE_SET_DEFAULT");

            for i in 32 .. 127 {
                inputter.assign_mode_command(mode_insert, std::str::from_utf8(&[i]).unwrap().to_string(), "INSERT_STRING");
                inputter.assign_mode_command(mode_overwrite, std::str::from_utf8(&[i]).unwrap().to_string(), "OVERWRITE_STRING");
                inputter.assign_mode_command(mode_cmd, std::str::from_utf8(&[i]).unwrap().to_string(), "INSERT_TO_CMDLINE");
            }

            inputter.assign_mode_command(mode_cmd, std::str::from_utf8(&[10]).unwrap().to_string(), "RUN_CUSTOM_COMMAND");
            inputter.assign_mode_command(mode_cmd, std::str::from_utf8(&[27]).unwrap().to_string(), "MODE_SET_DEFAULT");
            inputter.assign_mode_command(mode_cmd, std::str::from_utf8(&[127]).unwrap().to_string(), "CMDLINE_BACKSPACE");


            /////////////////////////////////
            // Rectmanager puts stdout in non-canonical mode,
            // so stdin will be char-by-char
            let stdout = io::stdout();
            let mut reader = io::stdin();
            let mut buffer;

            stdout.lock().flush().unwrap();
            ////////////////////////////////


            let mut do_push: bool;
            loop {
                buffer = [0;1];
                reader.read_exact(&mut buffer).unwrap();
                for character in buffer.iter() {
                    match c.try_lock() {
                        Ok(ref mut mutex) => {
                            do_push = true;

                            match &mutex.new_context {
                                Some(context) => {
                                    inputter.set_context(&context);
                                }
                                None => ()
                            }

                            mutex.new_context = None;

                            for (context, sequence, funcref) in mutex.new_input_sequences.drain(..) {
                                inputter.assign_mode_command(&context, sequence, &funcref);
                            }
                        }
                        Err(_e) => ()
                    }

                    match inputter.read_input(*character) {
                        Some((funcref, input_sequence)) => {
                            match c.try_lock() {
                                Ok(ref mut mutex) => {
                                    do_push = true;

                                    for (current_func, current_arg) in (mutex.function_queue).iter() {
                                        if *current_func == funcref && *current_arg == input_sequence {
                                            do_push = false;
                                            break;
                                        }
                                    }

                                    if do_push {
                                        (mutex.function_queue).push((funcref, input_sequence));
                                    }

                                }
                                Err(_e) => {
                                }
                            }
                        }
                        None => ()
                    }
                }

            }
        });

        let fps = 59.97;

        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);
        self.raise_flag(Flag::SETUP_DISPLAYS);

        let mut output: Result<(), Box<dyn Error>> = Ok(());
        while !self.flag_kill {
            match input_interface.try_lock() {
                Ok(ref mut mutex) => {

                    if mutex.flag_kill {
                        break;
                    }

                    if (mutex.function_queue).len() > 0 {
                        let (_current_func, _current_arg) = (mutex.function_queue).remove(0);
                        // Ignore input while waiting for the inputter to set new context.
                        match mutex.new_context {
                            Some(_) => { }
                            None => {
                                self.run_cmd_from_functionref(&_current_func, vec![_current_arg])?;
                            }
                        }
                    }

                    match &self.flag_input_context {
                        Some(context_key) => {
                            (mutex.new_context) = Some(context_key.to_string());
                        }
                        None => { }
                    }

                    self.flag_input_context = None;

                    for (context, sequence, funcref) in self.new_input_sequences.drain(..) {
                        (mutex.new_input_sequences).push((context, sequence, funcref));
                    }
                }
                Err(_e) => ()
            }

            match self.tick() {
                Ok(_) => {
                    thread::sleep(delay);
                }
                Err(boxed_error) => {
                    // To help debug ...
                    self.user_error_msg = Some(format!("{:?}", boxed_error));
                    //self.flag_kill = true;
                    //Err(Box::new(error))?;
                }
            }
        }

        self.kill();

        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), RectError> {
        self.rectmanager.kill()
    }

    fn unmap_structure(&mut self, structure_id: u64) {
        // Clear out any old mapping
        match self.structure_spans.get(&structure_id) {
            Some((span_i, span_f)) => {
                for i in *span_i .. *span_f {
                    match self.structure_map.get_mut(&i) {
                        Some(sid_hashset) => {
                            sid_hashset.remove(&structure_id);
                        }
                        None => {}
                    }
                }
            }
            None => {}
        }
    }

    fn set_structure_span(&mut self, structure_id: u64, new_span: (usize, usize)) {
        self.unmap_structure(structure_id);

        // update the span
        self.structure_spans.entry(structure_id)
            .and_modify(|span| *span = new_span)
            .or_insert(new_span);

        // update the map
        match self.structure_spans.get(&structure_id) {
            Some((span_i, span_f)) => {
                for i in *span_i .. *span_f {
                    self.structure_map.entry(i)
                        .or_insert(HashSet::new());

                    self.structure_map.entry(i)
                        .and_modify(|sid_set| { sid_set.insert(structure_id); });
                }
            }
            None => { } // Should be unreachable
        }
    }

    fn new_structure_handler(&mut self, index: usize, length: usize, handler: Box<dyn StructuredDataHandler>) -> u64 {
        let new_id = self.structure_id_gen;
        self.structure_id_gen += 1;
        self.structures.insert(new_id, handler);
        self.set_structure_span(new_id, (index, index + length));
        self.structure_validity.insert(new_id, true);

        new_id
    }

    fn remove_structure_handler(&mut self, handler_id: u64) {
        self.structures.remove(&handler_id);
        self.unmap_structure(handler_id);
        self.structure_spans.remove(&handler_id);
        self.structure_validity.remove(&handler_id);
    }

    fn shift_structure_handlers_after(&mut self, offset: usize, adjustment: isize) -> Vec<(u64, (usize, usize))> {
        let mut history: Vec<(u64, (usize, usize))>= Vec::new();
        let mut new_spans = Vec::new();
        for (sid, span) in self.structure_spans.iter_mut() {
            if span.0 >= offset {
                history.push((*sid, (span.0, span.1)));
                new_spans.push(
                    (
                        *sid,
                        (
                            ((span.0 as isize) + adjustment) as usize,
                            ((span.1 as isize) + adjustment) as usize
                        )
                    )
                );
            } else if span.1 > offset {
                history.push((*sid, (span.0, span.1)));
                new_spans.push(
                    (
                        *sid,
                        (
                            span.0,
                            ((span.1 as isize) + adjustment) as usize
                        )
                    )
                );
            }
        }
        for (sid, new_span) in new_spans.iter() {
            self.set_structure_span(*sid, *new_span);
        }

        history
    }

    fn get_visible_structured_data_handlers(&mut self, offset: usize, search_width: usize) -> Vec<((usize, usize), u64)> {
        let mut output = Vec::new();

        for (sid, span) in self.structure_spans.iter() {
            // If span starts after first point, but before last
            // if span ends after first point but before last
            // if span start before first point, but ends after
            if (span.0 >= offset && span.0 < offset + search_width)
            || (span.1 >= offset && span.1 < offset + search_width)
            || (span.0 <= offset && span.1 > offset) {
                output.push((*span, *sid));
            }
        }

        output
    }

    fn get_structured_data_handlers(&mut self, offset: usize) -> Vec<((usize, usize), u64)> {
        let mut output = Vec::new();

        match self.structure_map.get(&offset) {
            Some(ids) => {
                for sid in ids.iter() {
                    match self.structure_spans.get(sid) {
                        Some(span) => {
                            output.push((*span, *sid));
                        }
                        None => {}
                    }
                }

            }
            None => {}
        }

        // We want inner most structures first
        output.sort();
        output.reverse();

        output
    }

    fn run_structure_checks(&mut self, offset: usize) -> Vec<(u64, (usize, usize))> {
        let mut updated_structures = Vec::new();
        let mut working_bytes;
        let mut working_bytes_len;

        let mut difference: isize = 0;

        let mut was_valid: bool;
        let mut new_structure: Option<Box<dyn StructuredDataHandler>>;
        for (span, handler_id) in self.get_structured_data_handlers(offset).iter() {
            working_bytes_len = span.1 - span.0;
            working_bytes = self.get_chunk(span.0, working_bytes_len);
            new_structure = None;
            was_valid = self.structure_validity[handler_id];
            match self.structures.get_mut(handler_id) {
                Some(handler) => {
                    match handler.update(working_bytes) {
                        Ok(_) => {
                            self.structure_validity.entry(*handler_id).and_modify(|e| *e = true);
                        }
                        Err(e) => {
                            self.structure_validity.entry(*handler_id).and_modify(|e| *e = false);
                        }
                    }
                }
                None => ()
            }
            if was_valid != self.structure_validity[handler_id] {
                updated_structures.push((*handler_id, *span));
            }
        }

        updated_structures
    }

    fn increment_byte(&mut self, offset: usize) -> Result<(), EditorError> {
        let mut current_byte_offset = offset;
        if self.active_content.len() > current_byte_offset {
            let mut current_byte_value = self.active_content[current_byte_offset];
            let mut undo_bytes = vec![];

            loop {
                undo_bytes.insert(0, current_byte_value);
                if current_byte_value < 255 {

                    self.active_content[current_byte_offset] = current_byte_value + 1;
                    break;
                } else {
                    self.active_content[current_byte_offset] = 0;
                    if current_byte_offset > 0 {
                        current_byte_offset -= 1;
                    } else {
                        break;
                    }
                    current_byte_value = self.active_content[current_byte_offset];
                }
            }

            self.push_to_undo_stack(current_byte_offset, undo_bytes.len(), undo_bytes);
            Ok(())
        } else {
            Err(EditorError::OutOfRange(offset, self.active_content.len()))
        }
    }

    fn decrement_byte(&mut self, offset: usize) -> Result<(), EditorError> {
        let mut current_byte_offset = offset;

        if self.active_content.len() > current_byte_offset {
            let mut current_byte_value = self.active_content[current_byte_offset];

            let mut undo_bytes = vec![];

            loop {
                undo_bytes.insert(0, current_byte_value);
                if current_byte_value > 0 {
                    self.active_content[current_byte_offset] = current_byte_value - 1;
                    break;
                } else {
                    self.active_content[current_byte_offset] = 255;
                    if current_byte_offset > 0 {
                        current_byte_offset -= 1;
                    } else {
                        break;
                    }
                    current_byte_value = self.active_content[current_byte_offset];
                }
            }

            self.push_to_undo_stack(current_byte_offset, undo_bytes.len(), undo_bytes);
            Ok(())
        } else {
            Err(EditorError::OutOfRange(offset, self.active_content.len()))
        }
    }

    // ONLY to be used in insert_bytes and overwrite_bytes. nowhere else.
    fn _insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) {
        let mut is_ok = true;
        if offset < self.active_content.len() {
            for (i, new_byte) in new_bytes.iter().enumerate() {
                self.active_content.insert(offset + i, *new_byte);
            }
        } else if offset == self.active_content.len() {
            for new_byte in new_bytes.iter() {
                self.active_content.push(*new_byte);
            }
        } else {
            is_ok = false;
            #[cfg(debug_assertions)]
            {
                //TODO Debug error log
                //logg(Err(EditorError::OutOfRange(offset, self.active_content.len())));
            }
        }

        if is_ok {
            self.shift_structure_handlers_after(offset, new_bytes.len() as isize);
        }
    }

    // ONLY to be  used by remove_bytes and overwrite_bytes functions, nowhere else.
    fn _remove_bytes(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut output;
        if offset < self.active_content.len() {
            let mut removed_bytes = Vec::new();
            let adj_length = min(self.active_content.len() - offset, length);
            for i in 0..adj_length {
                removed_bytes.push(self.active_content.remove(offset));
            }
            output = removed_bytes;
        } else {
            output = vec![];

            #[cfg(debug_assertions)]
            {
                //TODO Debug error log
                //logg(Err(EditorError::OutOfRange(offset, self.active_content.len())));
            }
        }

        self.shift_structure_handlers_after(offset, 0 - (output.len() as isize));

        output
    }

    fn build_key_map() -> HashMap<&'static str, &'static str> {
        let mut key_map = HashMap::new();
        // Common control characters
        key_map.insert("BACKSPACE", "\x7F");
        key_map.insert("TAB", "\x09");
        key_map.insert("LINE_FEED", "\x0A");
        key_map.insert("RETURN", "\x0D");
        key_map.insert("ESCAPE", "\x1B");
        key_map.insert("ARROW_UP", "\x1B[A");
        key_map.insert("ARROW_LEFT", "\x1B[D");
        key_map.insert("ARROW_DOWN", "\x1B[B");
        key_map.insert("ARROW_RIGHT", "\x1B[C");
        key_map.insert("DELETE", "\x1B[3\x7e");

        // lesser control characters
        key_map.insert("NULL", "\x00");
        key_map.insert("STX", "\x01");
        key_map.insert("SOT", "\x02");
        key_map.insert("ETX", "\x03");
        key_map.insert("EOT", "\x04");
        key_map.insert("ENQ", "\x05");
        key_map.insert("ACK", "\x06");
        key_map.insert("BELL", "\x07");
        key_map.insert("VTAB", "\x0B");
        key_map.insert("FORM_FEED", "\x0C");
        key_map.insert("SHIFT_OUT", "\x0E");
        key_map.insert("SHIFT_IN", "\x0F");
        key_map.insert("DATA_LINK_ESCAPE", "\x10");
        key_map.insert("XON", "\x11");
        key_map.insert("CTRL+R", "\x12");
        key_map.insert("XOFF", "\x13");
        key_map.insert("DC4", "\x14");
        key_map.insert("NAK", "\x15");
        key_map.insert("SYN", "\x16");
        key_map.insert("ETB", "\x17");
        key_map.insert("CANCEL", "\x18");
        key_map.insert("EM", "\x19");
        key_map.insert("SUB", "\x1A");
        key_map.insert("FILE_SEPARATOR", "\x1C");
        key_map.insert("GROUP_SEPARATOR", "\x1D");
        key_map.insert("RECORD_SEPARATOR", "\x1E");
        key_map.insert("UNITS_EPARATOR", "\x1F");

        // Regular character Keys
        key_map.insert("ONE", "1");
        key_map.insert("TWO", "2");
        key_map.insert("THREE", "3");
        key_map.insert("FOUR", "4");
        key_map.insert("FIVE", "5");
        key_map.insert("SIX", "6");
        key_map.insert("SEVEN", "7");
        key_map.insert("EIGHT", "8");
        key_map.insert("NINE", "9");
        key_map.insert("ZERO", "0");
        key_map.insert("BANG", "!");
        key_map.insert("AT", "@");
        key_map.insert("OCTOTHORPE", "#");
        key_map.insert("DOLLAR", "$");
        key_map.insert("PERCENT", "%");
        key_map.insert("CARET", "^");
        key_map.insert("AMPERSAND", "&");
        key_map.insert("ASTERISK", "*");
        key_map.insert("PARENTHESIS_OPEN", "(");
        key_map.insert("PARENTHESIS_CLOSE", ")");
        key_map.insert("BRACKET_OPEN", "[");
        key_map.insert("BRACKET_CLOSE", "]");
        key_map.insert("BRACE_OPEN", "{");
        key_map.insert("BRACE_CLOSE", "}");
        key_map.insert("BAR", "|");
        key_map.insert("BACKSLASH", "\\");
        key_map.insert("COLON", ":");
        key_map.insert("SEMICOLON", ";");
        key_map.insert("QUOTE", "\"");
        key_map.insert("APOSTROPHE", "'");
        key_map.insert("LESSTHAN", "<");
        key_map.insert("GREATERTHAN", ">");
        key_map.insert("COMMA", ",");
        key_map.insert("PERIOD", ".");
        key_map.insert("SLASH", "/");
        key_map.insert("QUESTIONMARK", "?");
        key_map.insert("DASH", "-");
        key_map.insert("UNDERSCORE", "_");
        key_map.insert("SPACE", " ");
        key_map.insert("PLUS", "+");
        key_map.insert("EQUALS", "=");
        key_map.insert("TILDE", "~");
        key_map.insert("BACKTICK", "`");
        key_map.insert("A_UPPER", "A");
        key_map.insert("B_UPPER", "B");
        key_map.insert("C_UPPER", "C");
        key_map.insert("D_UPPER", "D");
        key_map.insert("E_UPPER", "E");
        key_map.insert("F_UPPER", "F");
        key_map.insert("G_UPPER", "G");
        key_map.insert("H_UPPER", "H");
        key_map.insert("I_UPPER", "I");
        key_map.insert("J_UPPER", "J");
        key_map.insert("K_UPPER", "K");
        key_map.insert("L_UPPER", "L");
        key_map.insert("M_UPPER", "M");
        key_map.insert("N_UPPER", "N");
        key_map.insert("O_UPPER", "O");
        key_map.insert("P_UPPER", "P");
        key_map.insert("Q_UPPER", "Q");
        key_map.insert("R_UPPER", "R");
        key_map.insert("S_UPPER", "S");
        key_map.insert("T_UPPER", "T");
        key_map.insert("U_UPPER", "U");
        key_map.insert("V_UPPER", "V");
        key_map.insert("W_UPPER", "W");
        key_map.insert("X_UPPER", "X");
        key_map.insert("Y_UPPER", "Y");
        key_map.insert("Z_UPPER", "Z");
        key_map.insert("A_LOWER", "a");
        key_map.insert("B_LOWER", "b");
        key_map.insert("C_LOWER", "c");
        key_map.insert("D_LOWER", "d");
        key_map.insert("E_LOWER", "e");
        key_map.insert("F_LOWER", "f");
        key_map.insert("G_LOWER", "g");
        key_map.insert("H_LOWER", "h");
        key_map.insert("I_LOWER", "i");
        key_map.insert("J_LOWER", "j");
        key_map.insert("K_LOWER", "k");
        key_map.insert("L_LOWER", "l");
        key_map.insert("M_LOWER", "m");
        key_map.insert("N_LOWER", "n");
        key_map.insert("O_LOWER", "o");
        key_map.insert("P_LOWER", "p");
        key_map.insert("Q_LOWER", "q");
        key_map.insert("R_LOWER", "r");
        key_map.insert("S_LOWER", "s");
        key_map.insert("T_LOWER", "t");
        key_map.insert("U_LOWER", "u");
        key_map.insert("V_LOWER", "v");
        key_map.insert("W_LOWER", "w");
        key_map.insert("X_LOWER", "x");
        key_map.insert("Y_LOWER", "y");
        key_map.insert("Z_LOWER", "z");


        key_map
    }
}

impl Editor for SbyteEditor {
    fn undo(&mut self) {
        let task = self.undo_stack.pop();
        match task {
            Some(_task) => {
                let redo_task = self.do_undo_or_redo(_task);
                self.redo_stack.push(redo_task);
            }
            None => ()
        }
    }

    fn redo(&mut self) {
        let task = self.redo_stack.pop();
        match task {
            Some(_task) => {
                let undo_task = self.do_undo_or_redo(_task);
                // NOTE: Not using self.push_to_undo_stack. don't want to clear the redo stack
                self.undo_stack.push(undo_task);
            }
            None => ()
        }
    }


    fn do_undo_or_redo(&mut self, task: (usize, usize, Vec<u8>)) -> (usize, usize, Vec<u8>) {
        let (offset, bytes_to_remove, bytes_to_insert) = task;

        self.cursor_set_offset(offset);

        let mut opposite_bytes_to_insert = vec![];
        let mut insert_length: usize = 0;
        if bytes_to_remove > 0 {
            let removed_bytes = self._remove_bytes(offset, bytes_to_remove);
            insert_length += removed_bytes.len();
            opposite_bytes_to_insert = removed_bytes;
        }

        let mut opposite_bytes_to_remove = 0;
        if bytes_to_insert.len() > 0 {
            opposite_bytes_to_remove = bytes_to_insert.len();
            self._insert_bytes(offset, bytes_to_insert);
        }
        self.run_structure_checks(offset);

        (offset, opposite_bytes_to_remove, opposite_bytes_to_insert)
    }

    fn push_to_undo_stack(&mut self, offset: usize, bytes_to_remove: usize, bytes_to_insert: Vec<u8>) {

        self.redo_stack.drain(..);
        let is_insert = bytes_to_remove == 0 && bytes_to_insert.len() > 0;
        let is_remove = bytes_to_remove > 0 && bytes_to_insert.len() == 0;
        let is_overwrite = !is_insert && !is_remove;

        let mut was_merged = false;
        match self.undo_stack.last_mut() {
            Some((next_offset, next_bytes_to_remove, next_bytes_to_insert)) => {
                let will_insert = *next_bytes_to_remove == 0 && next_bytes_to_insert.len() > 0;
                let will_remove = *next_bytes_to_remove > 0 && next_bytes_to_insert.len() == 0;
                let will_overwrite = !will_insert && !will_remove;

                if is_insert && will_insert {
                    if *next_offset == offset + bytes_to_insert.len() {
                        let mut new_bytes = bytes_to_insert.clone();
                        new_bytes.extend(next_bytes_to_insert.iter().copied());
                        *next_bytes_to_insert = new_bytes;
                        *next_offset = offset;
                        was_merged = true;
                    } else if *next_offset == offset {
                        next_bytes_to_insert.extend(bytes_to_insert.iter().copied());
                        was_merged = true;
                    }
                } else if is_remove && will_remove {
                    if *next_offset + *next_bytes_to_remove == offset {
                        *next_bytes_to_remove += bytes_to_remove;
                        was_merged = true;
                    }
                } else if is_overwrite && will_overwrite {
                }
            }
            None => ()
        }

        if !was_merged {
            self.undo_stack.push((offset, bytes_to_remove, bytes_to_insert));
        }
    }

    fn get_active_converter(&self) -> Box<dyn Converter> {
        match self.active_converter {
            ConverterRef::HEX => {
                Box::new(HexConverter {})
            }
            ConverterRef::BIN => {
                Box::new(BinaryConverter {})
            }
            ConverterRef::DEC => {
                Box::new(DecConverter {})
            }
            _ => {
                Box::new(HexConverter {})
            }
        }
    }

    fn replace(&mut self, search_for: Vec<u8>, replace_with: Vec<u8>) {
        let mut matches = self.find_all(&search_for);
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

    fn get_clipboard(&mut self) -> Vec<u8> {
        self.clipboard.clone()
    }

    fn copy_selection(&mut self) {
        let selected_bytes = self.get_selected();
        self.copy_to_clipboard(selected_bytes);
    }

    fn load_file(&mut self, file_path: &str) -> std::io::Result<()> {
        self.active_content = Vec::new();

        self.set_file_path(file_path);
        match File::open(file_path) {
            Ok(mut file) => {
                let file_length = match file.metadata() {
                    Ok(metadata) => {
                        metadata.len()
                    }
                    Err(e) => {
                        0
                    }
                };

                let mut buffer: Vec<u8> = vec![0; file_length as usize];
                file.read(&mut buffer)?;

                for byte in buffer.iter() {
                    self.active_content.push(*byte);
                }
            }
            Err(e) => {
                Err(e)?
            }
        }
        Ok(())
    }

    fn save(&mut self) -> Result<(), Box<dyn Error>> {
        match &self.active_file_path {
            Some(path) => {
                self.save_as(&path.to_string())?;
            }
            None => {
                Err(SbyteError::PathNotSet)?;
            }
        };

        Ok(())
    }

    fn save_as(&mut self, path: &str) -> std::io::Result<()> {
        match File::create(path) {
            Ok(mut file) => {
                file.write_all(self.active_content.as_slice())?;
                // TODO: Handle potential file system problems
                //file.sync_all();
            }
            Err(e) => {
                Err(e)?;
            }
        }

        Ok(())
    }

    fn set_file_path(&mut self, new_file_path: &str) {
        self.active_file_path = Some(new_file_path.to_string());
    }

    fn find_all(&self, search_for: &Vec<u8>) -> Vec<usize> {
        let mut output: Vec<usize> = Vec::new();

        let search_length = search_for.len();

        let mut i = 0;
        let mut j_offset;
        while i <= self.active_content.len() - search_length {
            j_offset = 0;
            for (j, test_byte) in search_for.iter().enumerate() {
                if self.active_content[i + j] != *test_byte {
                    break;
                }
                j_offset += 1;
            }
            if j_offset == search_length {
                output.push(i);
            }
            i += max(1, j_offset);
        }

        output
    }

    fn find_after(&self, pattern: &Vec<u8>, offset: usize) -> Option<usize> {
        //TODO: This could definitely be sped up.
        let matches = self.find_all(pattern);
        let mut output = None;
        let mut found = false;

        if matches.len() > 0 {
            for i in matches.iter() {
                if *i > offset {
                    output = Some(*i);
                    found = true;
                    break;
                }
            }
            if !found {
                output = Some(matches[0]);
            }
        }

        output
    }


    fn remove_bytes(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let removed_bytes = self._remove_bytes(offset, length);
        self.push_to_undo_stack(offset, 0, removed_bytes.clone());

        removed_bytes
    }


    fn remove_bytes_at_cursor(&mut self) -> Vec<u8> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();
        self.remove_bytes(offset, length)
    }


    fn insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) {
        let mut adj_byte_width = new_bytes.len();
        self._insert_bytes(offset, new_bytes);

        self.push_to_undo_stack(offset, adj_byte_width, vec![]);
    }

    fn overwrite_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) -> Vec<u8> {
        let position = self.cursor.get_offset();
        self.overwrite_bytes(position, new_bytes)
    }

    fn overwrite_bytes(&mut self, position: usize, new_bytes: Vec<u8>) -> Vec<u8> {
        let length = new_bytes.len();
        let mut removed_bytes = self._remove_bytes(position, length);

        self._insert_bytes(position, new_bytes);
        self.push_to_undo_stack(position, length, removed_bytes.clone());

        removed_bytes
    }

    fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) {
        let position = self.cursor.get_offset();
        self.insert_bytes(position, new_bytes);
    }

    fn get_selected(&mut self) -> Vec<u8> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();

        self.get_chunk(offset, length)
    }

    fn get_chunk(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        for i in min(offset, self.active_content.len()) .. min(self.active_content.len(), offset + length) {
            output.push(self.active_content[i]);
        }

        output
    }

    fn cursor_next_byte(&mut self) {
        let new_position = self.cursor.get_offset() + 1;
        self.cursor_set_offset(new_position);
    }

    fn cursor_prev_byte(&mut self) {
        if self.cursor.get_offset() != 0 {
            let new_position = self.cursor.get_offset() - 1;
            self.cursor_set_offset(new_position);
        }
    }

    fn cursor_increase_length(&mut self) {
        let new_length;
        if self.cursor.get_real_length() == -1 {
            new_length = 1;
        } else {
            new_length = self.cursor.get_real_length() + 1;
        }

        self.cursor_set_length(new_length);
    }

    fn cursor_decrease_length(&mut self) {
        let new_length;
        if self.cursor.get_real_length() == 1 {
            new_length = -1
        } else {
            new_length = self.cursor.get_real_length() - 1;
        }

        self.cursor_set_length(new_length);
    }

    fn cursor_set_offset(&mut self, new_offset: usize) {
        let adj_offset = min(self.active_content.len(), new_offset);
        self.cursor.set_offset(adj_offset);
    }

    fn cursor_set_length(&mut self, new_length: isize) {
        let adj_length;
        if self.cursor.get_real_offset() == self.active_content.len() && new_length > 0 {
            self.cursor.set_length(1);
        } else if new_length < 0 {
            self.cursor.set_length(max(new_length, 0 - self.cursor.get_real_offset() as isize));
        } else if new_length == 0 {
        } else {
            adj_length = min(new_length as usize, self.active_content.len() - self.cursor.get_real_offset()) as isize;
            self.cursor.set_length(adj_length);
        }
    }

    fn get_display_ratio(&mut self) -> u8 {
        let human_converter = HumanConverter {};
        let human_string_length = human_converter.encode(vec![65]).len();

        let active_converter = self.get_active_converter();
        let active_string_length = active_converter.encode(vec![65]).len();

        ((active_string_length / human_string_length) + 1) as u8
    }
}

impl VisualEditor for SbyteEditor {
    fn cursor_next_line(&mut self) {
        let new_offset = self.cursor.get_real_offset() + self.viewport.get_width();
        self.cursor_set_offset(new_offset);
    }

    fn cursor_prev_line(&mut self) {
        let viewport_width = self.viewport.get_width();
        let new_offset = self.cursor.get_real_offset() - min(self.cursor.get_real_offset(), viewport_width);
        self.cursor_set_offset(new_offset);
    }

    fn cursor_increase_length_by_line(&mut self) {
        let mut new_length: isize = self.cursor.get_real_length() + (self.viewport.get_width() as isize);

        if self.cursor.get_real_length() < 0 && new_length >= 0 {
            new_length += 1;
        }

        self.cursor_set_length(new_length);
    }

    fn cursor_decrease_length_by_line(&mut self) {
        let mut new_length: isize = self.cursor.get_real_length() - (self.viewport.get_width() as isize);
        if self.cursor.get_real_length() > 0 && new_length < 0 {
            new_length -= 1;
        }
        self.cursor_set_length(new_length);
    }

    fn adjust_viewport_offset(&mut self) {
        let width = self.viewport.get_width();
        let height = self.viewport.get_height();
        let screen_buffer_length = width * height;
        let mut adj_viewport_offset = self.viewport.get_offset();

        let adj_cursor_offset = if self.cursor.get_real_length() <= 0 {
            self.cursor.get_offset()
        } else {
            self.cursor.get_offset() + self.cursor.get_length() - 1
        };

        while adj_cursor_offset >= screen_buffer_length + adj_viewport_offset {
            adj_viewport_offset += width;
        }

        while adj_viewport_offset > adj_cursor_offset {
            if width > adj_viewport_offset {
                adj_viewport_offset = 0;
            } else {
                adj_viewport_offset -= width;
            }
        }

        self.viewport.set_offset(adj_viewport_offset);
    }
}

impl InConsole for SbyteEditor {
    fn tick(&mut self) -> Result<(), Box::<dyn Error>> {
        if !self.surpress_tick {

            self.check_resize();

            if self.check_flag(Flag::SETUP_DISPLAYS) {
                match self.setup_displays() {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::SetupFailed(error))?
                    }
                }
            }

            if self.check_flag(Flag::REMAP_ACTIVE_ROWS) {
                match self.remap_active_rows() {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::RemapFailed(error))?
                    }
                }
            }

            let len = self.rows_to_refresh.len();
            if len > 0 {
                let mut in_timeout = Vec::new();
                let mut y;
                while self.rows_to_refresh.len() > 0 {
                    y = self.rows_to_refresh.pop().unwrap();
                    if self.check_flag(Flag::UPDATE_ROW(y)) {
                        match self.set_row_characters(y) {
                            Ok(_) => {}
                            Err(error) => {
                                Err(SbyteError::RowSetFailed(error))?
                            }
                        }
                    } else {
                        in_timeout.push(y);
                    }
                }
                self.rows_to_refresh = in_timeout;
            }


            if self.check_flag(Flag::CURSOR_MOVED) {
                match self.apply_cursor() {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::ApplyCursorFailed(error))?
                    }
                }
            }


            match &self.user_error_msg {
                Some(msg) => {
                    self.display_user_error(msg.clone())?;
                    self.user_error_msg = None;

                    // Prevent any user msg from clobbering this msg
                    self.user_msg = None;
                    self.lower_flag(Flag::UPDATE_OFFSET);
                }
                None => {
                    if self.check_flag(Flag::DISPLAY_CMDLINE) {
                        self.display_command_line()?;
                    } else {
                        let tmp_usr_msg = self.user_msg.clone();
                        match tmp_usr_msg {
                            Some(msg) => {
                                self.display_user_message(msg.clone())?;
                                self.user_msg = None;
                                self.lower_flag(Flag::UPDATE_OFFSET);
                            }
                            None => {
                                if self.check_flag(Flag::UPDATE_OFFSET) {
                                    self.display_user_offset()?;
                                }
                            }
                        }
                    }
                }
            }

            match self.rectmanager.draw() {
                Ok(_) => {}
                Err(error) => {
                    Err(SbyteError::DrawFailed(error))?;
                }
            }
        }

        Ok(())
    }

    fn autoset_viewport_size(&mut self) {
        let full_height = self.rectmanager.get_height();
        let full_width = self.rectmanager.get_width();
        let meta_height = 1;

        let display_ratio = self.get_display_ratio() as f64;
        let r: f64 = (1f64 / display_ratio);
        let a: f64 = (1f64 - ( 1f64 / (r + 1f64)));
        let mut base_width = ((full_width as f64) * a) as usize;

        match self.locked_viewport_width {
            Some(locked_width) => {
                base_width = min(locked_width, base_width);
            }
            None => ()
        }

        self.viewport.set_size(
            base_width,
            full_height - meta_height
        );

        // adjust viewport
        let old_offset = self.viewport.get_offset();
        self.viewport.set_offset((old_offset / base_width) * base_width);


        self.active_row_map.drain();
        for i in 0 .. self.viewport.get_height() {
            self.active_row_map.insert(i, false);
        }
    }

    fn setup_displays(&mut self) -> Result<(), RectError> {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();

        self.autoset_viewport_size();
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();

        self.rectmanager.resize(self.rect_meta, full_width, 1)?;
        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            full_height - 1
        )?;

        let (bits_display, human_display) = self.rects_display;
        self.rectmanager.clear_children(bits_display)?;
        self.rectmanager.clear_children(human_display)?;

        self.arrange_displays()?;

        self.cell_dict.drain();
        self.row_dict.drain();

        let display_ratio = self.get_display_ratio() as usize;
        let width_bits;
        if display_ratio != 1 {
            width_bits = max(1, display_ratio - 1);
        } else {
            width_bits = display_ratio;
        }

        let mut _bits_row_id;
        let mut _bits_cell_id;
        let mut _human_row_id;
        let mut _human_cell_id;
        let mut _cells_hashmap;
        for y in 0..viewport_height {
            self.active_row_map.entry(y)
                .and_modify(|e| *e = false)
                .or_insert(false);

            _bits_row_id = self.rectmanager.new_rect(bits_display).ok().unwrap();

            self.rectmanager.resize(
                _bits_row_id,
                (viewport_width * display_ratio) - 1,
                1
            )?;

            self.rectmanager.set_position(_bits_row_id, 0, y as isize)?;

            _human_row_id = self.rectmanager.new_rect(human_display).ok().unwrap();
            self.rectmanager.resize(
                _human_row_id,
                viewport_width,
                1
            )?;
            self.rectmanager.set_position(
                _human_row_id,
                0,
                y as isize
            )?;

            self.row_dict.entry(y)
                .and_modify(|e| *e = (_bits_row_id, _human_row_id))
                .or_insert((_bits_row_id, _human_row_id));

            _cells_hashmap = self.cell_dict.entry(y).or_insert(HashMap::new());

            for x in 0 .. viewport_width {
                _bits_cell_id = self.rectmanager.new_rect(_bits_row_id).ok().unwrap();
                self.rectmanager.resize(
                    _bits_cell_id,
                    width_bits,
                    1
                )?;

                self.rectmanager.set_position(
                    _bits_cell_id,
                    (x * display_ratio) as isize,
                    0
                )?;

                _human_cell_id = self.rectmanager.new_rect(_human_row_id).ok().unwrap();

                self.rectmanager.set_position(
                    _human_cell_id,
                    x as isize,
                    0
                )?;
                self.rectmanager.resize(_human_cell_id, 1, 1)?;

                _cells_hashmap.entry(x as usize)
                    .and_modify(|e| *e = (_bits_cell_id, _human_cell_id))
                    .or_insert((_bits_cell_id, _human_cell_id));
            }
        }

        self.flag_force_rerow = true;

        self.raise_flag(Flag::CURSOR_MOVED);

        Ok(())
    }

    fn check_resize(&mut self) {
        if self.rectmanager.auto_resize() {
            self.is_resizing = true;

            // Viewport offset needs to be set to zero to ensure each line has the correct width
            self.viewport.set_offset(0);
            self.cursor_set_offset(0);

            self.raise_flag(Flag::SETUP_DISPLAYS);
            self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
            self.flag_force_rerow = true;
            self.is_resizing = false;
        }
    }

    fn arrange_displays(&mut self) -> Result<(), RectError> {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        let meta_height = 1;

        self.rectmanager.set_position(
            self.rect_meta,
            0,
            (full_height - meta_height) as isize
        )?;


        let display_height = full_height - meta_height;
        self.rectmanager.clear_characters(self.rect_display_wrapper)?;

        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            display_height
        )?;

        self.rectmanager.set_position(
            self.rect_display_wrapper,
            0,
            0
        )?;

        let display_ratio = self.get_display_ratio();
        let (bits_id, human_id) = self.rects_display;

        let bits_display_width = self.viewport.get_width() * display_ratio as usize;
        let human_display_width = self.viewport.get_width();
        let remaining_space = full_width - bits_display_width - human_display_width;


        let bits_display_x = remaining_space / 2;

        self.rectmanager.resize(bits_id, bits_display_width, display_height)?;
        self.rectmanager.set_position(bits_id, bits_display_x as isize, 0)?;

        // TODO: Fill in a separator

        //let human_display_x = (full_width - human_display_width) as isize;
        let human_display_x = (remaining_space / 2) + bits_display_width;

        self.rectmanager.resize(human_id, human_display_width, display_height)?;
        self.rectmanager.set_position(human_id, human_display_x as isize, 0)?;

        Ok(())
    }

    fn remap_active_rows(&mut self) -> Result<(), RectError> {
        let width = self.viewport.get_width();
        let height = self.viewport.get_height();
        let initial_y = (self.viewport.get_offset() / width) as isize;

        self.adjust_viewport_offset();
        let new_y = (self.viewport.get_offset() / width) as isize;

        let diff: usize;
        if new_y > initial_y {
            diff = (new_y - initial_y) as usize;
        } else {
            diff = (initial_y - new_y) as usize;
        }

        if diff > 0 || self.flag_force_rerow {
            if diff < height && !self.flag_force_rerow {
                // Don't rerender rendered rows. just shuffle them around
                {
                    let (bits, human) = self.rects_display;
                    self.rectmanager.shift_contents(
                        human,
                        0,
                        initial_y - new_y
                    )?;
                    self.rectmanager.shift_contents(
                        bits,
                        0,
                        initial_y - new_y
                    )?;
                }

                let mut new_rows_map = HashMap::new();
                let mut new_cells_map = HashMap::new();
                let mut new_active_map = HashMap::new();
                let mut from_y;
                if new_y < initial_y {
                    // Reassign the display_dicts to correspond to correct rows
                    for y in 0 .. height {

                        if diff > y {
                            from_y = height - ((diff - y) % height);
                        } else {
                            from_y = (y - diff) % height;
                        }

                        match self.row_dict.get(&from_y) {
                            Some((bits, human)) => {
                                new_rows_map.entry(y)
                                    .and_modify(|e| { *e = (*bits, *human)})
                                    .or_insert((*bits, *human));
                            }
                            None => ()
                        }

                        match self.cell_dict.get(&from_y) {
                            Some(cellhash) => {
                                new_cells_map.entry(y)
                                    .and_modify(|e| { *e = cellhash.clone()})
                                    .or_insert(cellhash.clone());
                            }
                            None => ()
                        }

                        if y < from_y {
                            // Moving row at bottom to top
                            match new_rows_map.get(&y) {
                                Some((bits, human)) => {
                                    self.rectmanager.set_position(*bits, 0, y as isize)?;
                                    self.rectmanager.set_position(*human, 0, y as isize)?;
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
                } else {
                    for y in 0 .. height {
                        from_y = (y + diff) % height;
                        match self.row_dict.get(&from_y) {
                            Some((bits, human)) => {
                                new_rows_map.entry(y)
                                    .and_modify(|e| { *e = (*bits, *human)})
                                    .or_insert((*bits, *human));

                            }
                            None => ()
                        }

                        match self.cell_dict.get(&from_y) {
                            Some(cellhash) => {
                                new_cells_map.entry(y)
                                    .and_modify(|e| { *e = cellhash.clone()})
                                    .or_insert(cellhash.clone());
                            }
                            None => ()
                        }

                        if from_y < y {
                            //Moving row at top to the bottom
                            match new_rows_map.get(&y) {
                                Some((bits, human)) => {
                                    self.rectmanager.set_position(*human, 0, y as isize)?;
                                    self.rectmanager.set_position(*bits, 0, y as isize)?;
                                }
                                None => ()
                            }
                            new_active_map.insert(y, false);
                        } else {
                            match self.active_row_map.get(&from_y) {
                                Some(needs_refresh) => {
                                    new_active_map.insert(y, *needs_refresh);
                                }
                                // *Shouldn't* happen
                                None => {
                                    new_active_map.insert(y, false);
                                }
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
                self.active_row_map.drain();
                for y in 0 .. height {
                    self.active_row_map.insert(y, false);
                }
            }

            let active_rows = self.active_row_map.clone();
            for (y, is_rendered) in active_rows.iter() {
                if !is_rendered {
                    self.raise_row_update_flag(*y + (new_y as usize));
                }
            }

            self.raise_flag(Flag::UPDATE_OFFSET);
        }

        self.flag_force_rerow = false;
        self.raise_flag(Flag::CURSOR_MOVED);

        Ok(())
    }

    fn set_row_characters(&mut self, absolute_y: usize) -> Result<(), RectError> {
        let viewport = &self.viewport;
        let active_converter = self.get_active_converter();
        let human_converter = HumanConverter {};
        let width = viewport.get_width();
        let offset = width * absolute_y;

        let structure_handlers = self.get_visible_structured_data_handlers(offset, width);
        let mut structured_cells_map = HashMap::new();
        let mut x;
        let mut y;
        for (span, sid) in structure_handlers.iter() {
            for i in span.0 .. span.1 {
                x = i % width;
                y = i / width;
                structured_cells_map.entry((x, y)).or_insert(self.structure_validity[sid]);
            }
        }

        let chunk = self.get_chunk(offset, width);
        let relative_y = absolute_y - (self.viewport.get_offset() / width);

        match self.cell_dict.get_mut(&relative_y) {
            Some(cellhash) => {

                for (_x, (rect_id_bits, rect_id_human)) in cellhash.iter_mut() {
                    self.rectmanager.clear_characters(*rect_id_human)?;
                    self.rectmanager.clear_characters(*rect_id_bits)?;
                }

                let mut tmp_bits;
                let mut tmp_bits_str;
                let mut tmp_human;
                let mut tmp_human_str;
                let mut in_structure;
                let mut structure_valid;
                for (x, byte) in chunk.iter().enumerate() {
                    tmp_bits = active_converter.encode_byte(*byte);
                    tmp_human = human_converter.encode_byte(*byte);

                    match structured_cells_map.get(&(x, absolute_y)) {
                        Some(is_valid) => {
                            in_structure = true;
                            structure_valid = *is_valid;
                        }
                        None => {
                            structure_valid = false;
                            in_structure = false;
                        }
                    }

                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            tmp_bits_str = match std::str::from_utf8(tmp_bits.as_slice()) {
                                Ok(valid) => {
                                    valid
                                }
                                Err(e) => {
                                    // Shouldn't Happen
                                    "."
                                }
                            };
                            tmp_human_str = match std::str::from_utf8(tmp_human.as_slice()) {
                                Ok(valid) => {
                                    valid
                                }
                                Err(e) => {
                                    "."
                                }
                            };

                            for (i, c) in tmp_human_str.chars().enumerate() {
                                self.rectmanager.set_character(*human, i as isize, 0, c);
                            }
                            for (i, c) in tmp_bits_str.chars().enumerate() {
                                self.rectmanager.set_character(*bits, i as isize, 0, c);
                            }

                            if in_structure {
                                self.rectmanager.set_underline_flag(*human)?;
                                self.rectmanager.set_underline_flag(*bits)?;
                            } else {
                                self.rectmanager.unset_underline_flag(*human)?;
                                self.rectmanager.unset_underline_flag(*bits)?;
                            }

                            if in_structure && !structure_valid {
                                self.rectmanager.set_fg_color(*human, RectColor::RED)?;
                                self.rectmanager.set_fg_color(*bits, RectColor::RED)?;
                            } else {
                                self.rectmanager.unset_color(*human)?;
                                self.rectmanager.unset_color(*bits)?;
                            }

                        }
                        None => { }
                    }
                }
            }
            None => { }
        }

        self.active_row_map.entry(relative_y)
            .and_modify(|e| {*e = true})
            .or_insert(true);

        Ok(())
    }

    fn display_user_offset(&mut self) -> Result<(), RectError> {
        let mut cursor_string = format!("{}", self.cursor.get_offset());

        if self.active_content.len() > 0 {
            let digit_count = (self.active_content.len() as f64).log10().ceil() as usize;
            let l = cursor_string.len();
            if l < digit_count {
                for _ in 0 .. (digit_count - l) {
                    cursor_string = format!("{}{}", " ", cursor_string);
                }
            }

        }

        let denominator = if self.active_content.len() == 0 {
            0
        } else {
            self.active_content.len() - 1
        };

        let cursor_len = self.cursor.get_length();
        let offset_display = if cursor_len == 1 {
                format!("Offset: {} / {}", cursor_string, denominator)
            } else {
                format!("Offset: {} ({}) / {}", cursor_string, cursor_len, denominator)

            };

        let meta_width = self.rectmanager.get_rect_width(self.rect_meta);

        let x = meta_width - offset_display.len();

        self.clear_meta_rect()?;

        self.rectmanager.set_string(self.rect_meta, x as isize, 0, &offset_display)?;

        Ok(())
    }

    fn clear_meta_rect(&mut self) -> Result<(), RectError> {
        self.rectmanager.clear_characters(self.rect_meta)?;
        self.rectmanager.clear_children(self.rect_meta)?;
        self.rectmanager.clear_effects(self.rect_meta)?;

        Ok(())
    }

    fn display_user_message(&mut self, msg: String) -> Result<(), RectError> {
        self.clear_meta_rect()?;
        self.rectmanager.set_string(self.rect_meta, 0, 0, &msg)?;
        self.rectmanager.set_bold_flag(self.rect_meta)?;
        self.rectmanager.set_fg_color(self.rect_meta, RectColor::BRIGHTCYAN)?;

        Ok(())
    }

    fn display_user_error(&mut self, msg: String) -> Result<(), RectError> {
        self.clear_meta_rect()?;
        self.rectmanager.set_string(self.rect_meta, 0, 0, &msg)?;
        self.rectmanager.set_fg_color(self.rect_meta, RectColor::RED)?;

        Ok(())
    }

    fn apply_cursor(&mut self) -> Result<(), RectError> {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let viewport_offset = self.viewport.get_offset();
        let cursor_offset = self.cursor.get_offset();
        let cursor_length = self.cursor.get_length();

        // First clear previously applied
        // (They may no longer exist, but that's ok)
        for (bits, human) in self.active_cursor_cells.drain() {
            self.rectmanager.unset_invert_flag(bits);
            self.rectmanager.unset_invert_flag(human);
        }

        let start = if cursor_offset < viewport_offset {
            viewport_offset
        } else {
            cursor_offset
        };

        let end = if cursor_offset + cursor_length > viewport_offset + (viewport_height * viewport_width) {
            viewport_offset + (viewport_height * viewport_width)
        } else {
            cursor_offset + cursor_length
        };

        let mut y;
        let mut x;
        for i in start .. end {
            y = (i - viewport_offset) / viewport_width;
            match self.cell_dict.get(&y) {
                Some(cellhash) => {
                    x = (i - viewport_offset) % viewport_width;
                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            self.rectmanager.set_invert_flag(*bits)?;
                            self.rectmanager.set_invert_flag(*human)?;
                            self.cells_to_refresh.insert((*bits, *human));
                            self.active_cursor_cells.insert((*bits, *human));
                        }
                        None => ()
                    }
                }
                None => ()
            }
        }

        Ok(())
    }

    fn display_command_line(&mut self) -> Result<(), RectError> {
        self.clear_meta_rect()?;
        let cmd = &self.commandline.get_register();
        // +1, because of the ":" at the start
        let cursor_x = self.commandline.get_cursor_offset() + 1;
        let cursor_id = self.rectmanager.new_rect(self.rect_meta).ok().unwrap();

        self.rectmanager.resize(cursor_id, 1, 1)?;
        self.rectmanager.set_position(cursor_id, cursor_x as isize, 0)?;
        self.rectmanager.set_invert_flag(cursor_id)?;

        if cursor_x < cmd.len() {
            let chr: String = cmd.chars().skip(cursor_x).take(1).collect();
            self.rectmanager.set_string(cursor_id, 0, 0, &chr)?;
        }

        self.rectmanager.set_string(self.rect_meta, 0, 0, &vec![":", cmd].join(""))?;

        Ok(())
    }

    fn flag_row_update_by_range(&mut self, range: std::ops::Range<usize>) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let first_active_row = range.start / viewport_width;
        let last_active_row = range.end / viewport_width;

        for y in first_active_row .. max(last_active_row + 1, first_active_row + 1) {
            self.raise_flag(Flag::UPDATE_ROW(y));
            self.raise_row_update_flag(y);
        }
    }

    fn flag_row_update_by_offset(&mut self, offset: usize) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let first_active_row = offset / viewport_width;

        for y in first_active_row .. first_active_row + viewport_height {
            self.raise_row_update_flag(y);
        }
    }

    fn raise_row_update_flag(&mut self, absolute_y: usize) {

        self.raise_flag(Flag::UPDATE_ROW(absolute_y));
        self.rows_to_refresh.push(absolute_y);
    }

    fn raise_flag(&mut self, key: Flag) {
        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, true))
            .or_insert((0, true));
    }

    fn lower_flag(&mut self, key: Flag) {
        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, false))
            .or_insert((0, false));
    }

    fn check_flag(&mut self, key: Flag) -> bool {
        let mut output = false;
        match self.display_flags.get_mut(&key) {
            Some((countdown, flagged)) => {
                if *countdown > 0 {
                    *countdown -= 1;
                } else {
                    output = *flagged;
                }
            }
            None => ()
        }

        if output {
            let mut new_timeout = 0;
            match self.display_flag_timeouts.get(&key) {
                Some(timeout) => {
                    new_timeout = *timeout;
                }
                None => { }
            }

            self.display_flags.entry(key)
                .and_modify(|e| *e = (new_timeout, false))
                .or_insert((new_timeout, false));
        }

        output
    }

    fn unlock_viewport_width(&mut self) {
        self.locked_viewport_width = None;
    }

    fn lock_viewport_width(&mut self, new_width: usize) {
        self.locked_viewport_width = Some(new_width);
    }
}

impl CommandInterface for SbyteEditor {
    fn ci_cursor_up(&mut self, repeat: usize) {
        let cursor_offset = self.cursor.get_offset();
        self.cursor_set_offset(cursor_offset);
        self.cursor_set_length(1);
        for _ in 0 .. repeat {
            self.cursor_prev_line();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::UPDATE_OFFSET);
        self.raise_flag(Flag::CURSOR_MOVED);
    }

    fn ci_cursor_down(&mut self, repeat: usize) {
        let end_of_cursor = self.cursor.get_offset() + self.cursor.get_length();
        self.cursor_set_length(1);
        self.cursor_set_offset(end_of_cursor - 1);
        for _ in 0 .. repeat {
            self.cursor_next_line();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::UPDATE_OFFSET);
        self.raise_flag(Flag::CURSOR_MOVED);
    }

    fn ci_cursor_left(&mut self, repeat: usize) {
        let cursor_offset = self.cursor.get_offset();
        self.cursor_set_offset(cursor_offset);
        self.cursor_set_length(1);
        for _ in 0 .. repeat {
            self.cursor_prev_byte();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::UPDATE_OFFSET);
        self.raise_flag(Flag::CURSOR_MOVED);

    }

    fn ci_cursor_right(&mut self, repeat: usize) {
        // Jump positon to the end of the cursor before moving it right
        let end_of_cursor = self.cursor.get_offset() + self.cursor.get_length();
        self.cursor_set_offset(end_of_cursor - 1);
        self.cursor_set_length(1);

        for _ in 0 .. repeat {
            self.cursor_next_byte();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_cursor_length_up(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.cursor_decrease_length_by_line();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_cursor_length_down(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.cursor_increase_length_by_line();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_cursor_length_left(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.cursor_decrease_length();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_cursor_length_right(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.cursor_increase_length();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_yank(&mut self) {
        self.copy_selection();
        self.cursor_set_length(1);
        self.raise_flag(Flag::CURSOR_MOVED);
    }

    fn ci_jump_to_position(&mut self, new_offset: usize) {
        self.cursor_set_length(1);
        self.cursor_set_offset(new_offset);

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_jump_to_next(&mut self, argument: Option<Vec<u8>>, repeat: usize) {
        let current_offset = self.cursor.get_offset();
        let mut next_offset = current_offset;
        let mut new_cursor_length = self.cursor.get_length();
        let mut new_user_msg = None;
        let mut new_user_error_msg = None;

        let option_pattern: Option<Vec<u8>> = match argument {
            Some(byte_pattern) => { // argument was given, use that
                Some(byte_pattern.clone())
            }
            None => { // No argument was given, check history
                match self.search_history.last() {
                    Some(byte_pattern) => {
                        Some(byte_pattern.clone())
                    }
                    None => {
                        None
                    }
                }
            }
        };

        match option_pattern {
            Some(raw_bytes) => {
                // First, convert utf8 bytes to a string...
                let string_rep = std::str::from_utf8(&raw_bytes).unwrap();
                // Then, convert the special characters in that string (eg, \x41 -> A)
                match self.string_to_bytes(string_rep.to_string()) {
                    Ok(byte_pattern) => {
                        self.search_history.push(raw_bytes.to_vec());
                        match self.find_after(&byte_pattern, current_offset) {
                            Some(new_offset) => {
                                new_cursor_length = byte_pattern.len();
                                next_offset = new_offset;
                                new_user_msg = Some(format!("Found \"{}\" at byte {}", string_rep, next_offset));
                            }
                            None => {
                                new_user_error_msg = Some(format!("Pattern \"{}\" not found", string_rep));
                            }
                        }
                    }
                    Err(e) => {
                        new_user_error_msg = Some(format!("Invalid pattern: {}", string_rep));
                    }
                }
            }
            None => {
                new_user_error_msg = Some("Need a pattern to search".to_string());
            }
        }

        self.user_msg = new_user_msg;
        self.user_error_msg = new_user_error_msg;

        self.cursor_set_length(new_cursor_length as isize);
        self.cursor_set_offset(next_offset);

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_delete(&mut self, repeat: usize) {
        let offset = self.cursor.get_offset();

        let repeat = self.grab_register(1);
        let mut removed_bytes = Vec::new();
        for _ in 0 .. repeat {
            removed_bytes.extend(self.remove_bytes_at_cursor().iter().copied());
        }
        self.copy_to_clipboard(removed_bytes);

        self.cursor_set_length(1);

        self.raise_flag(Flag::CURSOR_MOVED);
        self.flag_row_update_by_offset(offset);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_backspace(&mut self, repeat: usize) {
        let offset = self.cursor.get_offset();
        let adj_repeat = min(offset, repeat);

        self.ci_cursor_left(adj_repeat);
        // cast here is ok. repeat can't be < 0.
        self.cursor_set_length(adj_repeat as isize);
        self.ci_delete(1);
    }

    fn ci_undo(&mut self, repeat: usize) {
        let adj_repeat = min(self.undo_stack.len(), repeat);
        let current_viewport_offset = self.viewport.get_offset();

        for _ in 0 .. adj_repeat {
            self.undo();
            self.run_structure_checks(self.cursor.get_offset());
        }

        if adj_repeat > 1 {
            self.user_msg = Some(format!("Undone x{}", adj_repeat));
        } else if repeat == 1 {
            self.user_msg = Some("Undid last change".to_string());
        } else {
            self.user_msg = Some("Nothing to undo".to_string());
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        if self.viewport.get_offset() == current_viewport_offset {
            let start = self.viewport.get_offset() / self.viewport.get_width();
            let end = self.viewport.get_height() + start;
            for y in start .. end {
                self.raise_row_update_flag(y);
            }
        }
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_redo(&mut self, repeat: usize) {
        let current_viewport_offset = self.viewport.get_offset();

        for _ in 0 .. repeat {
            self.redo();
        }

        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
        if self.viewport.get_offset() == current_viewport_offset {
            let start = self.viewport.get_offset() / self.viewport.get_width();
            let end = self.viewport.get_height() + start;
            for y in start .. end {
                self.raise_row_update_flag(y);
            }
        }
        self.raise_flag(Flag::CURSOR_MOVED);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_insert_string(&mut self, argument: &str, repeat: usize) {
        match self.string_to_bytes(argument.to_string()) {
            Ok(converted_bytes) => {
                self.ci_insert_bytes(converted_bytes.clone(), repeat);
            }
            Err(e) => {
                self.user_error_msg = Some(format!("Invalid Pattern: {}", argument.clone()));
            }
        }
    }

    fn ci_insert_bytes(&mut self, bytes: Vec<u8>, repeat: usize) {
        let offset = self.cursor.get_offset();
        for _ in 0 .. repeat {
            self.insert_bytes_at_cursor(bytes.clone());
        }
        self.ci_cursor_right(bytes.len() * repeat);

        self.run_structure_checks(offset);
        self.flag_row_update_by_offset(offset);
        self.raise_flag(Flag::UPDATE_OFFSET);
    }

    fn ci_overwrite_string(&mut self, argument: &str, repeat: usize) {
        match self.string_to_bytes(argument.to_string()) {
            Ok(converted_bytes) => {
                self.ci_overwrite_bytes(converted_bytes.clone(), repeat);
            }
            Err(e) => {
                self.user_error_msg = Some(format!("Invalid Pattern: {}", argument.clone()));
            }
        }

    }
    fn ci_overwrite_bytes(&mut self, bytes: Vec<u8>, repeat: usize) {
        let offset = self.cursor.get_offset();
        for _ in 0 .. repeat {
            self.overwrite_bytes_at_cursor(bytes.clone());
            self.ci_cursor_right(bytes.len());
        }
        // TODO: This is almost certainly buggy
        // Manage structured data
        self.run_structure_checks(offset);

        self.cursor_set_length(1);
        self.raise_flag(Flag::CURSOR_MOVED);

        self.flag_row_update_by_range(offset..offset);
    }

    fn ci_increment(&mut self, repeat: usize) {
        let offset = self.cursor.get_offset();
        for _ in 0 .. repeat {
            match self.increment_byte(offset) {
                Err(EditorError::OutOfRange(n, l)) => {
                    break;
                }
                Ok(_) => {}
                Err(_) => {} // TODO
            }
        }
        self.run_structure_checks(offset);

        self.cursor_set_length(1);

        let mut suboffset: usize = 0;
        let mut chunk;
        while offset > suboffset {
            chunk = self.get_chunk(offset - suboffset, 1);
            if chunk.len() > 0 && (chunk[0] as u32) < (repeat >> (8 * suboffset)) as u32 {
                suboffset += 1;
            } else {
                break;
            }
        }

        self.flag_row_update_by_range(offset - suboffset .. offset);
        self.raise_flag(Flag::CURSOR_MOVED);
    }

    fn ci_decrement(&mut self, repeat: usize) {
        let offset = self.cursor.get_offset();
        for _ in 0 .. repeat {
            match self.decrement_byte(offset) {
                Err(EditorError::OutOfRange(n, l)) => {
                    break;
                }
                Ok(_) => {}
                Err(_) => {} // TODO
            }
        }
        self.run_structure_checks(offset);

        self.cursor_set_length(1);
        self.raise_flag(Flag::CURSOR_MOVED);

        let mut chunk;
        let mut suboffset: usize = 0;
        while offset > suboffset {
            chunk = self.get_chunk(offset - suboffset, 1);
            if chunk.len() > 0 && (chunk[0] as u32) > (repeat >> (8 * suboffset)) as u32 {
                suboffset += 1;
            } else {
                break;
            }
        }

        self.flag_row_update_by_range(offset - suboffset .. offset);
        self.raise_flag(Flag::CURSOR_MOVED);
    }
    fn ci_save(&mut self, path: Option<&str>) {
        match path {
            Some(string_path) => {
                match self.save_as(&string_path) {
                    Ok(_) => {
                        self.user_msg = Some(format!("Saved to file: {}", string_path))
                    }
                    Err(e) => {
                        self.user_error_msg = Some(format!("{:?}", e));
                    }
                }
            }
            None => {
                match self.save() {
                    Ok(_) => {
                        let file_path = self.active_file_path.as_ref().unwrap();
                        self.user_msg = Some(format!("Saved to file: {}", file_path));
                    }
                    Err(e) => {
                        self.user_error_msg = Some("No path specified".to_string());

                    }
                }
            }
        }
    }

    fn ci_lock_viewport_width(&mut self, new_width: usize) {
        self.lock_viewport_width(new_width);
        self.raise_flag(Flag::SETUP_DISPLAYS);
        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
    }

    fn ci_unlock_viewport_width(&mut self) {
        self.unlock_viewport_width();
        self.raise_flag(Flag::SETUP_DISPLAYS);
        self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
    }
}

impl Commandable for SbyteEditor {
    fn set_input_context(&mut self, context: &str) {
        self.flag_input_context = Some(context.to_string());
    }

    fn assign_line_command(&mut self, command_string: &str, function: &str) {
        self.line_commands.insert(command_string.to_string(), function.to_string());
    }

    fn try_command(&mut self, query: &str) -> Result<(), Box<dyn Error>> {
        // TODO: split words.
        let mut words = parse_words(query.to_string());
        if words.len() > 0 {
            let cmd = words.remove(0);
            let mut arguments: Vec<Vec<u8>> = vec![];

            for word in words.iter() {
                arguments.push(word.as_bytes().to_vec());
            }

            let funcref = match self.line_commands.get(&cmd) {
                Some(_funcref) => {
                   _funcref.to_string()
                }
                None => {
                    self.user_error_msg = Some(format!("Command not found: {}", cmd.clone()));
                    "NULL".to_string()
                }
            };

            self.run_cmd_from_functionref(&funcref, arguments)?;
        }

        Ok(())
    }

    fn run_cmd_from_functionref(&mut self, funcref: &str, arguments: Vec<Vec<u8>>) -> Result<(), Box<dyn Error>> {
        match funcref {
            "ASSIGN_INPUT" => {
                let mut is_ok = true;

                let new_funcref: String = match arguments.get(0) {
                    Some(_new_funcref) => {
                        std::str::from_utf8(_new_funcref).unwrap().to_string()
                    }
                    None => {
                        is_ok = false;
                        "".to_string()
                    }
                };

                let new_input_string: String = match arguments.get(1) {
                    Some(_new_inputs) => {
                        let tmp = std::str::from_utf8(_new_inputs).unwrap().to_string();
                        let key_map = SbyteEditor::build_key_map();
                        let mut output = "".to_string();
                        for word in tmp.split(",") {
                            match key_map.get(word) {
                                Some(seq) => {
                                    output += &seq.to_string();
                                }
                                None => () // TODO: ERROR
                            }
                        }
                        output
                    }
                    None => {
                        is_ok = false;
                        "".to_string()
                    }
                };

                if is_ok {
                    self.new_input_sequences.push(("DEFAULT".to_string(), new_input_string, new_funcref));
                }
            }

            "CURSOR_UP" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_up(repeat);
            }

            "CURSOR_DOWN" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_down(repeat);
            }

            "CURSOR_LEFT" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_left(repeat);
            }

            "CURSOR_RIGHT" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_right(repeat);
            }

            "CURSOR_LENGTH_UP" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_length_up(repeat)
            }

            "CURSOR_LENGTH_DOWN" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_length_down(repeat);
            }

            "CURSOR_LENGTH_LEFT" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_length_left(repeat);
            }

            "CURSOR_LENGTH_RIGHT" => {
                let repeat = self.grab_register(1);
                self.ci_cursor_length_right(repeat);
            }

            "JUMP_TO_REGISTER" => {
                let new_offset = max(0, self.grab_register(std::usize::MAX)) as usize;
                self.ci_jump_to_position(new_offset);
            }

            // TODO: look this logic over
            "JUMP_TO_NEXT" => {
                let repeat = self.grab_register(1);
                let pattern = match arguments.get(0) {
                    Some(bytes) => {
                       Some(bytes.clone())
                    }
                    None => {
                        None
                    }
                };
                self.ci_jump_to_next(pattern, repeat);
            }

            "CMDLINE_BACKSPACE" => {
                if self.commandline.is_empty() {
                    self.set_input_context("DEFAULT");
                    self.raise_flag(Flag::UPDATE_OFFSET);
                } else {
                    self.commandline.backspace();
                    self.raise_flag(Flag::DISPLAY_CMDLINE);
                }
            }

            "YANK" => {
                self.ci_yank();
            }

            "PASTE" => {
                let repeat = self.grab_register(1);
                let to_paste = self.get_clipboard();
                self.ci_insert_bytes(to_paste, repeat);
            }

            "DELETE" => {
                let repeat = self.grab_register(1);
                self.ci_delete(repeat);

            }

            "REMOVE_STRUCTURE" => {
                let offset = self.cursor.get_offset();
                let mut structures = self.get_structured_data_handlers(offset);

                match structures.first() {
                    Some((span, sid)) => {
                        self.remove_structure_handler(*sid);

                        self.flag_row_update_by_range(span.0..span.1);
                    }
                    None => {}
                }
            }

            "CREATE_BIG_ENDIAN_STRUCTURE" => {
                let prefix_width = self.cursor.get_length();
                let offset = self.cursor.get_offset();
                let prefix = self.get_chunk(offset, prefix_width);
                let data_width = BigEndianPrefixed::decode_prefix(prefix);
                let chunk = self.get_chunk(offset, prefix_width + data_width);
                match BigEndianPrefixed::read_in(chunk) {
                    Ok(new_structure) => {
                        self.new_structure_handler(
                            offset,
                            prefix_width + data_width,
                            Box::new(new_structure)
                        );
                        self.flag_row_update_by_range(offset..offset + prefix_width + data_width);
                    }
                    Err(e) => {
                        self.user_error_msg = Some(format!("Couldn't build structure: {:?}",e));
                    }
                }
            }

            "BACKSPACE" => {
                let repeat = min(self.cursor.get_offset(), max(1, self.grab_register(1)) as usize);
                self.ci_backspace(repeat);
            }

            "UNDO" => {
                let repeat = self.grab_register(1);
                self.ci_undo(repeat);
            }

            "REDO" => {
                let repeat = self.grab_register(1);
                self.ci_redo(repeat);
            }

            "MODE_SET_INSERT" => {
                self.clear_register();
                self.user_msg = Some("--INSERT--".to_string());
            }

            "MODE_SET_OVERWRITE" => {
                self.clear_register();
                self.user_msg = Some("--OVERWRITE--".to_string());
            }

            "MODE_SET_APPEND" => {
                self.clear_register();
                self.ci_cursor_right(1);
                self.user_msg = Some("--INSERT--".to_string());
            }

            "MODE_SET_DEFAULT" => {
                self.clear_register();
                self.clear_meta_rect()?;
                self.raise_flag(Flag::UPDATE_OFFSET);
                self.raise_flag(Flag::CURSOR_MOVED);
            }

            "MODE_SET_CMD" => {
                self.commandline.clear_register();
                self.raise_flag(Flag::DISPLAY_CMDLINE);
            }

            "MODE_SET_SEARCH" => {
                self.commandline.set_register("find ");
                self.display_command_line()?;
            }

            "MODE_SET_INSERT_SPECIAL" => {
                let cmdstring;
                match self.active_converter {
                    ConverterRef::BIN => {
                        cmdstring = "insert \\b";
                    }
                    ConverterRef::HEX => {
                        cmdstring = "insert \\x";
                    }
                    _ => {
                        cmdstring = "insert ";
                    }
                }
                self.commandline.set_register(cmdstring);
                self.display_command_line()?;
            }

            "MODE_SET_OVERWRITE_SPECIAL" => {
                let cmdstring;
                match self.active_converter {
                    ConverterRef::BIN => {
                        cmdstring = "overwrite \\b";
                    }
                    ConverterRef::HEX => {
                        cmdstring = "overwrite \\x";
                    }
                    _ => {
                        cmdstring = "overwrite ";
                    }
                }
                self.commandline.set_register(cmdstring);
                self.display_command_line()?;
            }

            "INSERT_STRING" => {
                let pattern = match arguments.get(0) {
                    Some(argument_bytes) => {
                        std::str::from_utf8(argument_bytes).unwrap()
                    }
                    None => {
                        ""
                    }
                };
                let repeat = self.grab_register(1);
                self.ci_insert_string(pattern, repeat);

            }

            "INSERT_RAW" => {
                let repeat = self.grab_register(1);
                let pattern = match arguments.get(0) {
                    Some(arg) => {
                        arg.clone()
                    }
                    None => {
                        vec![]
                    }
                };
                self.ci_insert_bytes(pattern, repeat);
            }

            "INSERT_TO_CMDLINE" => {
                match arguments.get(0) {
                    Some(argument_bytes) => {
                        let argument = std::str::from_utf8(argument_bytes).unwrap();
                        self.commandline.insert_to_register(argument);
                        self.commandline.move_cursor_right();
                        self.display_command_line()?;
                    }
                    None => ()
                }
            }

            "OVERWRITE_STRING" => {
                let pattern = match arguments.get(0) {
                    Some(argument_bytes) => {
                        std::str::from_utf8(argument_bytes).unwrap()
                    }
                    None => {
                        ""
                    }
                };
                let repeat = self.grab_register(1);
                self.ci_overwrite_string(pattern, repeat);

            }

            "OVERWRITE_RAW" => {
                let repeat = self.grab_register(1);
                let pattern = match arguments.get(0) {
                    Some(arg) => {
                        arg.clone()
                    }
                    None => {
                        vec![]
                    }
                };
                self.ci_overwrite_bytes(pattern, repeat);
            }

            "DECREMENT" => {
                let repeat = max(1, self.grab_register(1));
                self.ci_decrement(repeat);
            }

            "INCREMENT" => {
                let repeat = self.grab_register(1);
                self.ci_increment(repeat);
            }

            "RUN_CUSTOM_COMMAND" => {
                match self.commandline.apply_register() {
                    Some(new_command) => {
                        self.clear_meta_rect()?;
                        self.try_command(&new_command)?;
                    }
                    None => {
                    }
                };
            }

            "KILL" => {
                self.flag_kill = true;
            }

            "QUIT" => {
                //TODO in later version: Prevent quitting when there are unsaved changes
                self.flag_kill = true;
            }

            "SAVE" => {
                let path = match arguments.get(0) {
                    Some(byte_path) => {
                        Some(std::str::from_utf8(byte_path).unwrap())
                    }
                    None => {
                        None
                    }
                };

                self.ci_save(path);
            }

            "SAVEQUIT" => {
                self.ci_save(None);
                self.flag_kill = true;
            }

            "TOGGLE_CONVERTER" => {
                if self.active_converter == ConverterRef::BIN {
                    self.active_converter = ConverterRef::HEX;
                } else if self.active_converter == ConverterRef::HEX {
                    self.active_converter = ConverterRef::DEC;
                } else if self.active_converter == ConverterRef::DEC {
                    self.active_converter = ConverterRef::BIN;
                }
                self.raise_flag(Flag::SETUP_DISPLAYS);
                self.raise_flag(Flag::REMAP_ACTIVE_ROWS);
            }

            "SET_WIDTH" => {
                match arguments.get(0) {
                    Some(bytes) => {
                        let str_rep = std::str::from_utf8(bytes).unwrap();
                        match self.string_to_integer(str_rep) {
                            Ok(new_width) => {
                                self.ci_lock_viewport_width(new_width);
                            }
                            Err(_e) => {
                                //TODO
                            }
                        }
                    }
                    None => {
                        self.ci_unlock_viewport_width();
                    }
                }
            }

            "SET_REGISTER" => {
                self.clear_register();
                match arguments.get(0) {
                    Some(byterep) => {
                        let stringrep = std::str::from_utf8(&byterep).unwrap();
                        match self.string_to_integer(stringrep) {
                            Ok(n) => {
                                self.register = Some(n);
                            }
                            Err(_e) => ()

                        }
                    }
                    None =>()
                }
            }

            "APPEND_TO_REGISTER" => {
                match arguments.get(0) {
                    Some(argument) => {
                        // TODO: This is ridiculous. maybe make a nice wrapper for String (len 1) -> u8?
                        let string = std::str::from_utf8(&argument).unwrap();

                        let mut digit;
                        for (i, character) in string.chars().enumerate() {
                            if character.is_digit(10) {
                                digit = character.to_digit(10).unwrap() as usize;
                                self.append_to_register(digit);
                            }
                        }
                    }
                    None => ()
                }
            }

            "CLEAR_REGISTER" => {
                self.clear_register()
            }

            _ => {
                // Unknown
            }
        }

        Ok(())
    }

    fn grab_register(&mut self, default_if_unset: usize) -> usize {
        let output = match self.register {
            Some(n) => {
                n
            }
            None => {
                default_if_unset
            }
        };
        self.clear_register();

        output
    }

    fn clear_register(&mut self) {
        self.register = None;
    }

    fn append_to_register(&mut self, new_digit: usize) {
        self.register = match self.register {
            Some(mut n) => {
                n *= 10;
                n += new_digit;
                Some(n)
            }
            None => {
                Some(new_digit)
            }
        };
    }

    // Convert argument string to bytes.
    fn string_to_bytes(&self, input_string: String) -> Result<Vec<u8>, ConverterError> {
        let mut use_converter: Option<Box<dyn Converter>> = None;

        let mut input_bytes = input_string.as_bytes().to_vec();
        if input_bytes.len() > 2 {
            if input_bytes[0] == 92 {
                match input_bytes[1] {
                    98 => { // b
                        use_converter = Some(Box::new(BinaryConverter {}));
                    }
                    100 => { // d
                        use_converter = Some(Box::new(DecConverter {}));
                    }
                    120 => { // x
                        use_converter = Some(Box::new(HexConverter {}));
                    }
                    _ => { }
                }
            }
        }

        match use_converter {
            Some(converter) => {
                converter.decode(input_bytes.split_at(2).1.to_vec())
            }
            None => {
                Ok(input_string.as_bytes().to_vec())
            }
        }
    }

    fn string_to_integer(&self, input_string: &str) -> Result<usize, ConverterError> {
        let mut use_converter: Option<Box<dyn Converter>> = None;

        let mut input_bytes = input_string.to_string().as_bytes().to_vec();
        if input_bytes.len() > 2 {
            if input_bytes[0] == 92 {
                match input_bytes[1] {
                    98 => { // b
                        use_converter = Some(Box::new(BinaryConverter {}));
                    }
                    100 => { // d
                        use_converter = Some(Box::new(DecConverter {}));
                    }
                    120 => { // x
                        use_converter = Some(Box::new(HexConverter {}));
                    }
                    _ => { }
                }
            }
        }
        match use_converter {
            Some(converter) => {
                converter.decode_integer(input_bytes.split_at(2).1.to_vec())
            }
            None => {
                let mut output = 0;
                let mut digit;
                for (i, character) in input_string.chars().enumerate() {
                    output *= 10;
                    if character.is_digit(10) {
                        digit = character.to_digit(10).unwrap() as usize;
                        output += digit;
                    }
                }
                Ok(output)
            }
        }
    }
}


// TODO: Consider quotes, apostrophes  and escapes
fn parse_words(input_string: String) -> Vec<String> {
    let mut output = Vec::new();

    let mut delimiters = HashMap::new();
    delimiters.insert(' ', ' ');
    delimiters.insert('"', '"');
    delimiters.insert('\'', '\'');

    let mut working_word: String = "".to_string();
    let mut opener: Option<char> = None;
    let mut is_escaped = false;
    for (i, c) in input_string.chars().enumerate() {
        match opener {
            Some(o_c) => {
                if !is_escaped {
                    if c == '\\' {
                        is_escaped = true;
                    }
                    match delimiters.get(&c) {
                        Some(test_opener) => {
                            if *test_opener == o_c {
                                opener = None;
                                if working_word.len() > 0 {
                                    output.push(working_word.clone());
                                }
                                working_word = "".to_string();
                            }
                        }
                        None => {
                            working_word.push(c);
                        }
                    }
                } else {
                    working_word.push(c);
                    is_escaped = false;
                }
            }
            None => {
                if c == '\\' {
                    is_escaped = true;
                }

                if c != ' ' {

                    if c != '"' && c != '\'' {
                        opener = Some(' ');
                        working_word.push(c);
                    } else {
                        opener = Some(c);
                    }
                }
            }
        }
    }
    if working_word.len() > 0 {
        output.push(working_word.clone());
    }

    output
}

#[cfg (test)]
mod tests {
    use super::*;
    #[test]
    fn test_initializes_empty() {
        let mut editor = SbyteEditor::new();
        // Ok to kill for the test, we don't care about the
        // visuals at the moment
        editor.kill();

        assert_eq!(editor.active_content.as_slice(), []);
    }

    #[test]
    fn test_insert_bytes() {
        let mut editor = SbyteEditor::new();
        // Ok to kill for the test, we don't care about the
        // visuals at the moment
        editor.kill();

        editor.insert_bytes(0, vec![65]);
        assert_eq!(editor.active_content.as_slice(), [65]);

        // inserting out of range should ignore insertion
        editor.insert_bytes(10, vec![65]);
        assert_eq!(editor.active_content.as_slice(), [65]);
    }

    #[test]
    fn test_remove_bytes() {
        let mut editor = SbyteEditor::new();
        // Ok to kill for the test, we don't care about the
        // visuals at the moment
        editor.kill();
        editor.insert_bytes(0, vec![65]);


        assert_eq!(editor.remove_bytes(0, 1), vec![65]);
        assert_eq!(editor.active_content.as_slice(), []);
        assert_eq!(editor.remove_bytes(1000, 300), vec![]);
    }

    #[test]
    fn test_yanking() {
        let mut editor = SbyteEditor::new();
        // Ok to kill for the test, we don't care about the
        // visuals at the moment
        editor.kill();
        editor.insert_bytes(0, vec![65, 66, 67, 68]);

        editor.make_selection(1, 3);
        assert_eq!(editor.get_selected().as_slice(), [66, 67, 68]);

        editor.copy_selection();
        assert_eq!(editor.get_clipboard().as_slice(), [66, 67, 68]);
    }

    #[test]
    fn test_find() {
        let mut editor = SbyteEditor::new();
        // Ok to kill for the test, we don't care about the
        // visuals at the moment
        editor.kill();
        editor.insert_bytes(0, vec![65, 66, 0, 0, 65, 65, 66, 65]);

        let found = editor.find_all(&vec![65, 66]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], 0);
        assert_eq!(found[1], 5);

        assert_eq!(editor.find_after(&vec![65, 66], 2), Some(5));

    }
}
