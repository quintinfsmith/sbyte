use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::{time, thread};
use std::sync::{Mutex, Arc};
use wrecked::{RectManager, logg, RectColor};

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

use editor::{Editor, EditorError};
use editor::editor_cursor::Cursor;
use editor::converter::{HumanConverter, BinaryConverter, HexConverter, Converter, ConverterRef, ConverterError};
use visual_editor::*;
use visual_editor::viewport::ViewPort;
use commandable::Commandable;
use commandable::inputter::Inputter;
use commandable::inputter::function_ref::FunctionRef;
use command_line::CommandLine;
use inconsole::*;
use structured::*;


pub struct SbyteEditor {
    //Editor
    clipboard: Vec<u8>,
    active_content: Vec<u8>,
    active_file_path: Option<String>,
    cursor: Cursor,
    active_converter: ConverterRef,
    undo_stack: Vec<(usize, usize, Option<Vec<u8>>, Vec<(u64, (usize, usize))>)>, // Position, bytes to remove, bytes to insert
    redo_stack: Vec<(usize, usize, Option<Vec<u8>>, Vec<(u64, (usize, usize))>)>, // Position, bytes to remove, bytes to insert


    // Commandable
    commandline: CommandLine,
    line_commands: HashMap<String, FunctionRef>,
    register: isize,
    register_isset: bool,

    // VisualEditor
    viewport: ViewPort,

    // InConsole
    rectmanager: RectManager,
    active_row_map: HashMap<usize, bool>,
    flag_kill: bool,
    flag_force_rerow: bool,
    ready: bool,

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
    cell_dict: HashMap<usize, HashMap<usize, (usize, usize)>>,

    search_history: Vec<String>,

    structure_id_gen: u64,
    structures: HashMap<u64, Box<dyn StructuredDataHandler>>,
    structure_spans: HashMap<u64, (usize, usize)>,
    structure_map: HashMap<usize, HashSet<u64>>,
    structure_validity: HashMap<u64, bool>
}

impl SbyteEditor {
    pub fn new() -> SbyteEditor {
        let mut rectmanager = RectManager::new();
        let (width, height) = rectmanager.get_rect_size(0).ok().unwrap();
        let id_display_wrapper = rectmanager.new_rect(Some(0));
        let id_display_bits = rectmanager.new_rect(
            Some(id_display_wrapper)
        );
        let id_display_human = rectmanager.new_rect(
            Some(id_display_wrapper)
        );
        let id_rect_meta = rectmanager.new_rect(Some(0));

        let mut output = SbyteEditor {
            clipboard: Vec::new(),
            active_content: Vec::new(),
            active_file_path: None,
            cursor: Cursor::new(),
            active_converter: ConverterRef::HEX,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            register: 0,
            register_isset: false,

            viewport: ViewPort::new(width, height),

            line_commands: HashMap::new(),
            commandline: CommandLine::new(),
            rectmanager: rectmanager,

            active_row_map: HashMap::new(),
            flag_kill: false,
            flag_force_rerow: false,
            ready: false,

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
            cell_dict: HashMap::new(),

            search_history: Vec::new(),

            structure_id_gen: 0,
            structures: HashMap::new(),
            structure_spans: HashMap::new(),
            structure_validity: HashMap::new(),
            structure_map: HashMap::new()
        };

        output.assign_line_command("q".to_string(), FunctionRef::KILL);
        output.assign_line_command("w".to_string(), FunctionRef::SAVE);
        output.assign_line_command("wq".to_string(), FunctionRef::SAVEKILL);
        output.assign_line_command("find".to_string(), FunctionRef::JUMP_TO_NEXT);
        output.assign_line_command("insert".to_string(), FunctionRef::INSERT);
        output.assign_line_command("overwrite".to_string(), FunctionRef::OVERWRITE);

        output
    }

    pub fn main(&mut self) {
        let function_refs: Arc<Mutex<Vec<(FunctionRef, u8)>>> = Arc::new(Mutex::new(Vec::new()));

        let c = function_refs.clone();
        let mut _input_daemon = thread::spawn(move || {
            let mut inputter = Inputter::new();
            inputter.assign_mode_command(0,"\x1B[A".to_string(), FunctionRef::NULL);
            inputter.assign_mode_command(0,"\x1B[B".to_string(), FunctionRef::NULL);
            inputter.assign_mode_command(0,"\x1B[C".to_string(), FunctionRef::NULL);
            inputter.assign_mode_command(0,"\x1B[D".to_string(), FunctionRef::NULL);

            inputter.assign_mode_command(0, "=".to_string(), FunctionRef::TOGGLE_CONVERTER);
            inputter.assign_mode_command(0, "j".to_string(), FunctionRef::CURSOR_DOWN);
            inputter.assign_mode_command(0, "k".to_string(), FunctionRef::CURSOR_UP);
            inputter.assign_mode_command(0, "h".to_string(), FunctionRef::CURSOR_LEFT);
            inputter.assign_mode_command(0, "l".to_string(), FunctionRef::CURSOR_RIGHT);

            inputter.assign_mode_command(0, "J".to_string(), FunctionRef::CURSOR_LENGTH_DOWN);
            inputter.assign_mode_command(0, "K".to_string(), FunctionRef::CURSOR_LENGTH_UP);
            inputter.assign_mode_command(0, "H".to_string(), FunctionRef::CURSOR_LENGTH_LEFT);
            inputter.assign_mode_command(0, "L".to_string(), FunctionRef::CURSOR_LENGTH_RIGHT);

            inputter.assign_mode_command(0, "!".to_string(), FunctionRef::CREATE_BIG_ENDIAN_STRUCTURE);
            inputter.assign_mode_command(0, "@".to_string(), FunctionRef::REMOVE_STRUCTURE);

            for i in 0 .. 10 {
                inputter.assign_mode_command(0, std::str::from_utf8(&[i + 48]).unwrap().to_string(), FunctionRef::APPEND_TO_REGISTER);
            }

            inputter.assign_mode_command(0, "G".to_string(), FunctionRef::JUMP_TO_REGISTER);
            inputter.assign_mode_command(0, "/".to_string(), FunctionRef::MODE_SET_SEARCH);
            inputter.assign_mode_command(0, std::str::from_utf8(&[27]).unwrap().to_string(), FunctionRef::CLEAR_REGISTER);
            inputter.assign_mode_command(0, "x".to_string(), FunctionRef::DELETE);
            inputter.assign_mode_command(0, "u".to_string(), FunctionRef::UNDO);
            inputter.assign_mode_command(0, std::str::from_utf8(&[18]).unwrap().to_string(), FunctionRef::REDO);

            inputter.assign_mode_command(0, "i".to_string(), FunctionRef::MODE_SET_INSERT);
            inputter.assign_mode_command(0, "I".to_string(), FunctionRef::MODE_SET_INSERT_SPECIAL);
            inputter.assign_mode_command(0, "O".to_string(), FunctionRef::MODE_SET_OVERWRITE_SPECIAL);
            inputter.assign_mode_command(0, "a".to_string(), FunctionRef::MODE_SET_APPEND);
            inputter.assign_mode_command(0, "o".to_string(), FunctionRef::MODE_SET_OVERWRITE);
            inputter.assign_mode_command(0, ":".to_string(), FunctionRef::MODE_SET_CMD);

            inputter.assign_mode_command(0, "+".to_string(), FunctionRef::INCREMENT);
            inputter.assign_mode_command(0, "-".to_string(), FunctionRef::DECREMENT);

            inputter.assign_mode_command(1, std::str::from_utf8(&[27]).unwrap().to_string(), FunctionRef::MODE_SET_MOVE);
            inputter.assign_mode_command(2, std::str::from_utf8(&[27]).unwrap().to_string(), FunctionRef::MODE_SET_MOVE);

            for i in 32 .. 127 {
                inputter.assign_mode_command(1, std::str::from_utf8(&[i]).unwrap().to_string(), FunctionRef::INSERT);
                inputter.assign_mode_command(2, std::str::from_utf8(&[i]).unwrap().to_string(), FunctionRef::OVERWRITE);
                inputter.assign_mode_command(3, std::str::from_utf8(&[i]).unwrap().to_string(), FunctionRef::INSERT_TO_CMDLINE);
            }

            inputter.assign_mode_command(3, std::str::from_utf8(&[10]).unwrap().to_string(), FunctionRef::RUN_CUSTOM_COMMAND);
            inputter.assign_mode_command(3, std::str::from_utf8(&[27]).unwrap().to_string(), FunctionRef::MODE_SET_MOVE);

            inputter.assign_mode_command(0, std::str::from_utf8(&[127]).unwrap().to_string(), FunctionRef::BACKSPACE);
            inputter.assign_mode_command(3, std::str::from_utf8(&[127]).unwrap().to_string(), FunctionRef::CMDLINE_BACKSPACE);

            inputter.set_context_key(FunctionRef::MODE_SET_MOVE, 0);
            inputter.set_context_key(FunctionRef::RUN_CUSTOM_COMMAND, 0); // Switch back to move mode after calling cmd
            inputter.set_context_key(FunctionRef::MODE_SET_INSERT, 1);
            inputter.set_context_key(FunctionRef::MODE_SET_APPEND, 1);
            inputter.set_context_key(FunctionRef::MODE_SET_OVERWRITE, 2);
            inputter.set_context_key(FunctionRef::MODE_SET_CMD, 3);
            inputter.set_context_key(FunctionRef::MODE_SET_SEARCH, 3);
            inputter.set_context_key(FunctionRef::MODE_SET_INSERT_SPECIAL, 3);
            inputter.set_context_key(FunctionRef::MODE_SET_OVERWRITE_SPECIAL, 3);

            /////////////////////////////////
            // Rectmanager puts stdout in non-canonical mode,
            // so stdin will be char-by-char
            let stdout = io::stdout();
            let mut reader = io::stdin();
            let mut buffer;

            stdout.lock().flush().unwrap();
            ////////////////////////////////


            let mut do_push: bool;
            while true {
                buffer = [0;1];
                reader.read_exact(&mut buffer).unwrap();
                for character in buffer.iter() {
                    match inputter.read_input(*character) {
                        Some((funcref, input_byte)) => {
                            match c.try_lock() {
                                Ok(ref mut mutex) => {
                                    do_push = true;
                                    for (current_func, current_arg) in mutex.iter() {
                                        if *current_func == funcref && *current_arg == input_byte {
                                            do_push = false;
                                            break;
                                        }
                                    }

                                    if do_push {
                                        mutex.push((funcref, input_byte));
                                    }
                                }
                                Err(e) => {
                                    //logg(e.to_string());
                                }
                            }
                        }
                        None => {
                            ()
                        }
                    }
                }
            }
        });


        let fps = 30.0;

        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);
        self.setup_displays();

        while ! self.flag_kill {
            match function_refs.try_lock() {
                Ok(ref mut mutex) => {
                    if mutex.len() > 0 {
                        let (_current_func, _current_arg) = mutex.remove(0);
                        // Convert the u8 byte to a Vec<String> to fit the arguments data type
                        let args = vec![std::str::from_utf8(&[_current_arg]).unwrap().to_string()];

                        self.run_cmd_from_functionref(_current_func, args);
                    }
                }
                Err(e) => {
                }
            }

            self.tick();
            thread::sleep(delay);
        }
        self.kill();
    }

    pub fn kill(&mut self) {
        self.rectmanager.kill();
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

    fn run_structure_checks(&mut self, offset: usize) {
        let mut working_bytes;
        let mut working_bytes_len;

        let mut difference: isize = 0;

        let mut new_structure: Option<Box<dyn StructuredDataHandler>>;
        for (span, handler_id) in self.get_structured_data_handlers(offset).iter() {
            working_bytes_len = span.1 - span.0;
            working_bytes = self.get_chunk(span.0, working_bytes_len);
            new_structure = None;
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
        }
    }

    fn increment_byte(&mut self, offset: usize) -> Result<(), EditorError> {
        let mut current_byte_offset = offset;
        let mut current_byte_value = self.active_content[current_byte_offset];
        let mut undo_bytes = vec![];

        while true {
            if current_byte_value < 255 {
                undo_bytes.insert(0, current_byte_value);

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

        self.push_to_undo_stack(current_byte_offset, undo_bytes.len(), Some(undo_bytes), vec![]);

        Ok(())
    }

    fn decrement_byte(&mut self, offset: usize) -> Result<(), EditorError> {
        let mut current_byte_offset = offset;
        let mut current_byte_value = self.active_content[current_byte_offset];

        let mut undo_bytes = vec![];

        while true {
            if current_byte_value > 0 {
                undo_bytes.insert(0, current_byte_value);
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

        self.push_to_undo_stack(current_byte_offset, undo_bytes.len(), Some(undo_bytes), vec![]);

        Ok(())
    }

    // ONLY to be used in insert_bytes and overwrite_bytes. nowhere else.
    fn _insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) -> Result<(), EditorError> {
        let output;
        if offset < self.active_content.len() {
            for (i, new_byte) in new_bytes.iter().enumerate() {
                self.active_content.insert(offset + i, *new_byte);
            }
            output = Ok(());
        } else if offset == self.active_content.len() {
            for new_byte in new_bytes.iter() {
                self.active_content.push(*new_byte);
            }
            output = Ok(());
        } else {
            output = Err(EditorError::OutOfRange);
        }

        output
    }
    // ONLY to be  used by remove_bytes and overwrite_bytes functions, nowhere else.
    fn _remove_bytes(&mut self, offset: usize, length: usize) -> Result<Vec<u8>, EditorError> {
        if (offset < self.active_content.len()) {
            let mut removed_bytes = Vec::new();
            let adj_length = min(self.active_content.len() - offset, length);
            for i in 0..adj_length {
                removed_bytes.push(self.active_content.remove(offset));
            }
            Ok(removed_bytes)
        } else {
            Err(EditorError::OutOfRange)
        }
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
            None => {
            }
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
            None => {
            }
        }
    }


    fn do_undo_or_redo(&mut self, task: (usize, usize, Option<Vec<u8>>, Vec<(u64, (usize, usize))>)) -> (usize, usize, Option<Vec<u8>>, Vec<(u64, (usize, usize))>) {
        let (offset, bytes_to_remove, bytes_to_insert, handler_spans) = task;

        self.cursor_set_offset(offset);

        let mut opposite_bytes_to_insert = None;
        let mut insert_length: usize = 0;
        if bytes_to_remove > 0 {
            opposite_bytes_to_insert = match self._remove_bytes(offset, bytes_to_remove) {
                Ok(some_bytes) => {
                    insert_length += some_bytes.len();
                    Some(some_bytes)
                }
                Err(e) => None
            }
        }

        let mut opposite_bytes_to_remove = 0;
        match bytes_to_insert {
            Some(bytes) => {
                opposite_bytes_to_remove = bytes.len();
                self._insert_bytes(offset, bytes);
            }
            None => ()
        }

        let mut redo_handlers = Vec::new();

        for (sid, oldspan) in handler_spans.iter() {
            self.structure_spans.entry(*sid)
                .and_modify(|span| {
                    redo_handlers.push((*sid, (span.0, span.1)));
                    *span = (oldspan.0, oldspan.1);
                });
        }

        self.run_structure_checks(offset);

        (offset, opposite_bytes_to_remove, opposite_bytes_to_insert, redo_handlers)
    }

    fn push_to_undo_stack(&mut self, offset: usize, bytes_to_remove: usize, bytes_to_insert: Option<Vec<u8>>, structure_handler_states: Vec<(u64, (usize, usize))>) {
        self.redo_stack.drain(..);
        self.undo_stack.push((offset, bytes_to_remove, bytes_to_insert, structure_handler_states));
    }

    fn get_active_converter(&self) -> Box<dyn Converter> {
        match self.active_converter {
            ConverterRef::HEX => {
                Box::new(HexConverter {})
            }
            ConverterRef::BIN => {
                Box::new(BinaryConverter {})
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

    fn load_file(&mut self, file_path: String) {
        self.active_content = Vec::new();

        self.set_file_path(file_path.clone());
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
                file.read(&mut buffer);

                for byte in buffer.iter() {
                    self.active_content.push(*byte);
                }
            }
            Err(e) => {}
        }
    }

    fn save_file(&mut self) {
        match &self.active_file_path {
            Some(path) => {
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
            None =>  ()
        }

    }

    fn set_file_path(&mut self, new_file_path: String) {
        self.active_file_path = Some(new_file_path);
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

        if matches.len() > 0 {
            for i in matches.iter() {
                if *i > offset {
                    output = Some(*i);
                    break;
                }
            }
        }

        output
    }


    fn remove_bytes(&mut self, offset: usize, length: usize) -> Result<Vec<u8>, EditorError> {
        let output = self._remove_bytes(offset, length);

        let handlers_history = self.shift_structure_handlers_after(offset, 0 - (length as isize));
        match output {
            Ok(old_bytes) => {
                self.push_to_undo_stack(offset, 0, Some(old_bytes.clone()), handlers_history);



                Ok(old_bytes)
            }
            Err(e) => {
                Err(e)
            }
        }
    }


    fn remove_bytes_at_cursor(&mut self) -> Result<Vec<u8>, EditorError> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();
        self.remove_bytes(offset, length)
    }


    fn insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) -> Result<(), EditorError> {
        let mut adj_byte_width = new_bytes.len();
        let output = self._insert_bytes(offset, new_bytes);

        let handlers_history = self.shift_structure_handlers_after(offset, adj_byte_width as isize);
        self.push_to_undo_stack(offset, adj_byte_width, None, handlers_history);

        output
    }

    fn overwrite_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) -> Result<Vec<u8>, EditorError> {
        let position = self.cursor.get_offset();
        self.overwrite_bytes(position, new_bytes)
    }

    fn overwrite_bytes(&mut self, position: usize, new_bytes: Vec<u8>) -> Result<Vec<u8>, EditorError> {
        let length = new_bytes.len();
        let mut output = self._remove_bytes(position, length);
        match output {
            Ok(old_bytes) => {
                self._insert_bytes(position, new_bytes);
                self.push_to_undo_stack(position, length, Some(old_bytes.clone()), vec![]);



                Ok(old_bytes)
            }
            Err(e) => {
                Err(e)
            }
        }
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
        for i in offset .. min(self.active_content.len(), offset + length) {
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
            if (width > adj_viewport_offset) {
                adj_viewport_offset = 0;
            } else {
                adj_viewport_offset -= width;
            }
        }

        self.viewport.set_offset(adj_viewport_offset);
    }
}

impl InConsole for SbyteEditor {
    fn tick(&mut self) {
        self.check_resize();
        let do_draw = self.flag_refresh_full
            || self.flag_refresh_display
            || self.flag_refresh_meta
            || self.cells_to_refresh.len() > 0
            || self.rows_to_refresh.len() > 0;

        if self.flag_refresh_full {
            self.rectmanager.flag_refresh(0);
            self.flag_refresh_full = false;
            self.flag_refresh_display = false;
            self.flag_refresh_meta = false;
            self.cells_to_refresh.drain();
            self.rows_to_refresh.drain();
        }

        if self.flag_refresh_display {
            self.rectmanager.flag_refresh(self.rect_display_wrapper);
            self.flag_refresh_display = false;
            self.cells_to_refresh.drain();
            self.rows_to_refresh.drain();
        }

        for (_bits_id, _human_id) in self.cells_to_refresh.iter() {
            self.rectmanager.flag_refresh(*_bits_id);
            self.rectmanager.flag_refresh(*_human_id);
        }

        for (_bits_id, _human_id) in self.rows_to_refresh.iter() {
            self.rectmanager.flag_refresh(*_bits_id);
            self.rectmanager.flag_refresh(*_human_id);
        }

        self.cells_to_refresh.drain();
        self.rows_to_refresh.drain();

        if self.flag_refresh_meta {
            self.flag_refresh_meta = false;
            self.rectmanager.flag_refresh(self.rect_meta);
        }

        if do_draw {
            self.rectmanager.draw(0);
        }
    }

    fn autoset_viewport_size(&mut self) {
        let full_height = self.rectmanager.get_height();
        let full_width = self.rectmanager.get_width();
        //let meta_height = self.rectmanager.get_rect_size(self.rect_meta).ok().unwrap().1;
        let meta_height = 1;

        let display_ratio = self.get_display_ratio() as f64;
        let r: f64 = (1f64 / display_ratio);
        let a: f64 = (1f64 - ( 1f64 / (r + 1f64)));
        let base_width = (full_width as f64) * a;

        self.viewport.set_size(
            base_width as usize,
            full_height - meta_height
        );

        self.active_row_map.drain();
        for i in 0 .. self.viewport.get_height() {
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

        self.flag_force_rerow = true;
        self.remap_active_rows();

        self.apply_cursor();

        self.flag_refresh_full = true;
    }

    fn check_resize(&mut self) {
        if self.rectmanager.auto_resize() {
            self.is_resizing = true;
            // Viewport offset needs to be set to zero to ensure each line has the correct width
            self.viewport.set_offset(0);
            self.cursor_set_offset(0);
            self.setup_displays();
            self.flag_force_rerow = true;
            self.remap_active_rows();
            self.is_resizing = false;
        }
    }

    fn arrange_displays(&mut self) {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        let meta_height = 1;

        self.rectmanager.set_position(
            self.rect_meta,
            0,
            (full_height - meta_height) as isize
        );


        let display_height = full_height - meta_height;
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
        if new_y > initial_y {
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
                                    self.rectmanager.set_position(*bits, 0, y as isize);
                                    self.rectmanager.set_position(*human, 0, y as isize);
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
                if ! is_rendered {
                    self.set_row_characters(*y + (new_y as usize));
                }
            }

            self.set_offset_display();
            self.flag_refresh_display = true;
        }
        self.flag_force_rerow = false;
    }

    fn set_row_characters(&mut self, absolute_y: usize) {
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
                    self.rectmanager.clear(*rect_id_human);
                    self.rectmanager.clear(*rect_id_bits);
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
                    };

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
                            self.rectmanager.set_string(*human, 0, 0, tmp_human_str);
                            self.rectmanager.set_string(*bits, 0, 0, tmp_bits_str);
                            if in_structure {
                                self.rectmanager.set_underline_flag(*human);
                                self.rectmanager.set_underline_flag(*bits);
                            } else {
                                self.rectmanager.unset_underline_flag(*human);
                                self.rectmanager.unset_underline_flag(*bits);
                            }

                            if in_structure && !structure_valid {
                                self.rectmanager.set_fg_color(*human, RectColor::RED);
                                self.rectmanager.set_fg_color(*bits, RectColor::RED);
                            } else {
                                self.rectmanager.unset_fg_color(*human);
                                self.rectmanager.unset_fg_color(*bits);
                            }

                            //if self.active_converter == ConverterRef::BIN {
                            //    self.rectmanager.set_bg_color(*human, 3);
                            //    self.rectmanager.set_bg_color(*bits, 2);
                            //} else {
                            //    self.rectmanager.set_bg_color(*human, 5);
                            //    self.rectmanager.set_bg_color(*bits, 6);
                            //}
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

    fn set_offset_display(&mut self) {
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

        self.rectmanager.clear(self.rect_meta);
        let meta_width = self.rectmanager.get_rect_width(self.rect_meta);
        let x = meta_width - offset_display.len();
        self.rectmanager.set_string(self.rect_meta, x as isize, 0, &offset_display);

        self.flag_refresh_meta = true;
    }

    fn display_user_message(&mut self) {

    }

    fn apply_cursor(&mut self) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let viewport_offset = self.viewport.get_offset();
        let cursor_offset = self.cursor.get_offset();
        let cursor_length = self.cursor.get_length();

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

    fn remove_cursor(&mut self) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let viewport_offset = self.viewport.get_offset();
        let cursor_offset = self.cursor.get_offset();
        let cursor_length = self.cursor.get_length();

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
            if y < viewport_height {
                match self.cell_dict.get(&y) {
                    Some(cellhash) => {
                        x = (i - viewport_offset) % viewport_width;
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

    fn draw_cmdline(&mut self) {
        self.rectmanager.clear(self.rect_meta);
        self.rectmanager.empty(self.rect_meta);
        let cmd = &self.commandline.get_register();
        // +1, because of the ":" at the start
        let cursor_x = self.commandline.get_cursor_offset() + 1;
        let cursor_id = self.rectmanager.new_rect(Some(self.rect_meta));
        self.rectmanager.resize(cursor_id, 1, 1);
        self.rectmanager.set_position(cursor_id, cursor_x as isize, 0);
        self.rectmanager.set_invert_flag(cursor_id);
        if cursor_x < cmd.len() {
            let chr: String = cmd.chars().skip(cursor_x).take(1).collect();
            self.rectmanager.set_string(cursor_id, 0, 0, &chr);
        }

        self.rectmanager.set_string(self.rect_meta, 0, 0, ":");
        self.rectmanager.set_string(self.rect_meta, 1, 0, cmd);

        self.flag_refresh_meta = true;
    }

}

impl Commandable for SbyteEditor {
    fn assign_line_command(&mut self, command_string: String, function: FunctionRef) {
        self.line_commands.insert(command_string, function);
    }

    fn try_command(&mut self, query: String) {
        // TODO: split words.
        let mut words = parse_words(query);
        if words.len() > 0 {
            let cmd = words.remove(0);
            let mut funcref = FunctionRef::NULL;
            match self.line_commands.get(&cmd) {
                Some(_funcref) => {
                    funcref = *_funcref;
                }
                None => ()
            };

            self.run_cmd_from_functionref(funcref, words);
        }
    }

    fn run_cmd_from_functionref(&mut self, funcref: FunctionRef, arguments: Vec<String>) {
        match funcref {
            FunctionRef::CURSOR_UP => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let cursor_offset = self.cursor.get_offset();
                self.cursor_set_offset(cursor_offset);
                self.cursor_set_length(1);
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_prev_line();
                }
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_DOWN => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let repeat = self.grab_register(1);
                let end_of_cursor = self.cursor.get_offset() + self.cursor.get_length();
                self.cursor_set_length(1);
                self.cursor_set_offset(end_of_cursor - 1);
                for _ in 0 .. repeat {
                    self.cursor_next_line();
                }
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_LEFT => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_prev_byte();
                }
                self.cursor_set_length(1);
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_RIGHT => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();

                // Jump positon to the end of the cursor before moving it right
                let end_of_cursor = self.cursor.get_offset() + self.cursor.get_length();
                self.cursor_set_offset(end_of_cursor - 1);
                self.cursor_set_length(1);

                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_next_byte();
                }

                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_LENGTH_UP => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_decrease_length_by_line();
                }
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_LENGTH_DOWN => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_increase_length_by_line();
                }
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_LENGTH_LEFT => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_decrease_length();
                }
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CURSOR_LENGTH_RIGHT => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.cursor_increase_length();
                }
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::APPEND_TO_REGISTER => {
                match arguments.get(0) {
                    Some(argument) => {
                        // TODO: This is ridiculous. maybe make a nice wrapper for String (len 1) -> u8?
                        let digit = argument.chars().next().unwrap().to_digit(10).unwrap() as isize;
                        self.append_to_register(digit);
                    }
                    None => ()
                }
            }
            FunctionRef::CLEAR_REGISTER => {
                self.clear_register()
            }
            FunctionRef::JUMP_TO_REGISTER => {
                let current_offset = self.viewport.get_offset();
                self.remove_cursor();
                self.cursor_set_length(1);
                let new_offset = max(0, self.grab_register(std::isize::MAX)) as usize;
                self.cursor_set_offset(new_offset);
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::JUMP_TO_NEXT => {
                let current_offset = self.cursor.get_offset();
                let mut next_offset = current_offset;
                let mut new_cursor_length = self.cursor.get_length();

                match arguments.get(0) {
                    Some(pattern) => { // argument was given, use that
                        match self.string_to_bytes(pattern.to_string()) {
                            Ok(bytes) => {
                                self.search_history.push(pattern.clone());
                                match self.find_after(&bytes, current_offset) {
                                    Some(new_offset) => {
                                        new_cursor_length = bytes.len();
                                        next_offset = new_offset;
                                    }
                                    None => ()
                                }
                            }
                            Err(e) => {}
                        }
                    }
                    None => { // No argument was given, check history
                        match self.search_history.last() {
                            Some(pattern) => {
                                match self.string_to_bytes(pattern.to_string()) {
                                    Ok(bytes) => {
                                        match self.find_after(&bytes, current_offset) {
                                            Some(new_offset) => {
                                                new_cursor_length = bytes.len();
                                                next_offset = new_offset;
                                            }
                                            None => ()
                                        }
                                    }
                                    Err(e) => {}
                                }
                            }
                            None => ()
                        }
                    }
                }

                self.remove_cursor();
                self.cursor_set_length(new_cursor_length as isize);
                self.cursor_set_offset(next_offset);
                self.remap_active_rows();
                self.apply_cursor();
                self.set_offset_display();
                if self.viewport.get_offset() != current_offset {
                    self.flag_refresh_display = true;
                }
            }
            FunctionRef::CMDLINE_BACKSPACE => {
                self.commandline.backspace();
                self.draw_cmdline();
            }
            FunctionRef::DELETE => {
                let offset = self.cursor.get_offset();

                let repeat = self.grab_register(1);
                let mut removed_bytes = Vec::new();
                for _ in 0 .. repeat {
                    match self.remove_bytes_at_cursor() {
                        Ok(bytes) => {
                            removed_bytes.extend(bytes.iter().copied());
                        }
                        Err(e) => { }
                    }
                }

                self.run_structure_checks(offset);


                self.remove_cursor();
                self.cursor_set_length(1);
                self.apply_cursor();

                let viewport_width = self.viewport.get_width();
                let viewport_height = self.viewport.get_height();
                let active_row = offset / viewport_width;
                let viewport_line = self.viewport.get_offset() / viewport_width;

                for y in active_row .. viewport_line + viewport_height {
                    self.set_row_characters(y);
                }
                self.set_offset_display();
            }

            FunctionRef::REMOVE_STRUCTURE => {
                let offset = self.cursor.get_offset();
                let mut structures = self.get_structured_data_handlers(offset);

                match structures.first() {
                    Some((span, sid)) => {
                        self.remove_structure_handler(*sid);

                        let viewport_width = self.viewport.get_width();
                        let viewport_height = self.viewport.get_height();

                        let active_row = self.viewport.get_offset() / viewport_width;
                        let span_offset_a = (span.0 / viewport_width);
                        let span_offset_b = (span.1 / viewport_width);
                        for y in span_offset_a .. max(span_offset_b, span_offset_a + 1) + 1 {
                            self.set_row_characters(y);
                        }

                    }
                    None => {}
                }
            }

            FunctionRef::CREATE_BIG_ENDIAN_STRUCTURE => {
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

                        let viewport_width = self.viewport.get_width();
                        let viewport_height = self.viewport.get_height();

                        let viewport_row = self.viewport.get_offset() / viewport_width;
                        let first_row = offset / viewport_width;
                        let last_row = (offset + prefix_width + (data_width as usize)) / viewport_width;

                        for y in max(viewport_row, first_row) .. min(viewport_row + viewport_height, max(last_row, first_row + 1) + 1) {
                            self.set_row_characters(y);
                        }
                    }
                    Err(e) => {
                    }
                }

            }

            FunctionRef::BACKSPACE => {
                if self.cursor.get_offset() > 0 {
                    self.run_cmd_from_functionref(FunctionRef::CURSOR_LEFT, arguments.clone());
                    self.run_cmd_from_functionref(FunctionRef::DELETE, arguments.clone());
                }
            }

            FunctionRef::UNDO => {
                let current_viewport_offset = self.viewport.get_offset();
                self.remove_cursor();

                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.undo();
                }

                self.remap_active_rows();
                if self.viewport.get_offset() == current_viewport_offset {
                    let viewport_width = self.viewport.get_width();
                    let viewport_height = self.viewport.get_height();
                    let active_row = self.cursor.get_offset() / viewport_width;
                    let viewport_line = self.viewport.get_offset() / viewport_width;

                    for y in active_row .. viewport_line + viewport_height {
                        self.set_row_characters(y);
                    }
                }
                self.apply_cursor();
                self.set_offset_display();
            }

            FunctionRef::REDO => {
                let current_viewport_offset = self.viewport.get_offset();
                self.remove_cursor();

                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.redo();
                }

                self.remap_active_rows();
                if self.viewport.get_offset() == current_viewport_offset {
                    let viewport_width = self.viewport.get_width();
                    let viewport_height = self.viewport.get_height();
                    let active_row = self.cursor.get_offset() / viewport_width;
                    let viewport_line = self.viewport.get_offset() / viewport_width;

                    for y in active_row .. viewport_line + viewport_height {
                        self.set_row_characters(y);
                    }
                }
                self.apply_cursor();
                self.set_offset_display();
            }
            FunctionRef::MODE_SET_INSERT => {
                self.clear_register();
            }
            FunctionRef::MODE_SET_APPEND => {
                self.clear_register();
                self.run_cmd_from_functionref(FunctionRef::CURSOR_RIGHT, arguments);
            }
            FunctionRef::MODE_SET_MOVE => {
                self.clear_register();
                self.rectmanager.unset_bg_color(self.rect_meta);
            }
            FunctionRef::MODE_SET_CMD => {
                self.commandline.clear_register();
            }
            FunctionRef::MODE_SET_SEARCH => {
                self.commandline.set_register("find ".to_string());
                self.draw_cmdline();
            }
            FunctionRef::MODE_SET_INSERT_SPECIAL => {
                let cmdstring;
                match self.active_converter {
                    ConverterRef::BIN => {
                        cmdstring = "insert \\b".to_string();
                    }
                    ConverterRef::HEX => {
                        cmdstring = "insert \\x".to_string();
                    }
                    _ => {
                        cmdstring = "insert ".to_string();
                    }
                }
                self.commandline.set_register(cmdstring);
                self.draw_cmdline();
            }
            FunctionRef::MODE_SET_OVERWRITE_SPECIAL => {
                let cmdstring;
                match self.active_converter {
                    ConverterRef::BIN => {
                        cmdstring = "overwrite \\b".to_string();
                    }
                    ConverterRef::HEX => {
                        cmdstring = "overwrite \\x".to_string();
                    }
                    _ => {
                        cmdstring = "overwrite ".to_string();
                    }
                }
                self.commandline.set_register(cmdstring);
                self.draw_cmdline();
            }
            FunctionRef::INSERT => {
                let offset = self.cursor.get_offset();

                match arguments.get(0) {
                    Some(argument) => {
                        match self.string_to_bytes(argument.to_string()) {
                            Ok(bytes) => {
                                let repeat = self.grab_register(1);
                                if repeat > 0 {

                                    for _ in 0 .. repeat {
                                        self.insert_bytes_at_cursor(bytes.clone());
                                        self.run_cmd_from_functionref(FunctionRef::CURSOR_RIGHT, arguments.clone());
                                    }

                                    self.run_structure_checks(offset);

                                    let viewport_width = self.viewport.get_width();
                                    let viewport_height = self.viewport.get_height();
                                    let first_active_row = offset / viewport_width;
                                    let last_active_row = (self.viewport.get_offset() / viewport_width) + viewport_height;
                                    for y in first_active_row .. last_active_row {
                                        self.set_row_characters(y);
                                    }
                                    self.set_offset_display();
                                }
                            }
                            Err(e) => {
                                // TODO: Display converter error in meta display
                            }
                        }
                    }
                    None => ()
                }
            }
            FunctionRef::INSERT_TO_CMDLINE => {
                match arguments.get(0) {
                    Some(argument) => {
                        self.commandline.insert_to_register(argument.to_string());
                        self.commandline.move_cursor_right();
                        self.draw_cmdline();
                    }
                    None => ()
                }
            }
            FunctionRef::OVERWRITE => {
                let offset = self.cursor.get_offset();

                match arguments.get(0) {
                    Some(argument) => {
                        match self.string_to_bytes(argument.to_string()) {
                            Ok(bytes) => {
                                let repeat = self.grab_register(1);

                                let mut overwritten_bytes: Vec<u8> = Vec::new();
                                for _ in 0 .. repeat {
                                    self.overwrite_bytes_at_cursor(bytes.clone());
                                    self.run_cmd_from_functionref(FunctionRef::CURSOR_RIGHT, arguments.clone());
                                }


                                // Manage structured data
                                self.run_structure_checks(offset);


                                self.remove_cursor();
                                self.cursor_set_length(1);
                                self.apply_cursor();

                                let viewport_width = self.viewport.get_width();
                                let first_active_row = offset / viewport_width;
                                let last_active_row = (offset + 1) / viewport_width;

                                for y in first_active_row .. last_active_row + 1 {
                                    self.set_row_characters(y);
                                }
                            }
                            Err(e) => {
                                // TODO: Display converter error in meta display
                            }
                        }
                    }
                    None => ()
                }
            }
            FunctionRef::DECREMENT => {
                let offset = self.cursor.get_offset();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.decrement_byte(offset);
                }
                self.run_structure_checks(offset);

                self.remove_cursor();
                self.cursor_set_length(1);
                self.apply_cursor();

                let viewport_width = self.viewport.get_width();
                let first_active_row = offset / viewport_width;
                let last_active_row = (offset + 1) / viewport_width;

                for y in first_active_row .. last_active_row + 1 {
                    self.set_row_characters(y);
                }
            }
            FunctionRef::INCREMENT => {
                let offset = self.cursor.get_offset();
                let repeat = self.grab_register(1);
                for _ in 0 .. repeat {
                    self.increment_byte(offset);
                }
                self.run_structure_checks(offset);

                self.remove_cursor();
                self.cursor_set_length(1);
                self.apply_cursor();

                let viewport_width = self.viewport.get_width();
                let first_active_row = offset / viewport_width;
                let last_active_row = (offset + 1) / viewport_width;

                for y in first_active_row .. last_active_row + 1 {
                    self.set_row_characters(y);
                }
            }
            FunctionRef::RUN_CUSTOM_COMMAND => {
                match self.commandline.apply_register() {
                    Some(new_command) => {
                        self.rectmanager.clear(self.rect_meta);
                        self.rectmanager.empty(self.rect_meta);

                        self.try_command(new_command);
                    }
                    None => ()
                };
            }
            FunctionRef::KILL => {
                self.flag_kill = true;
            }
            FunctionRef::SAVE => {
                //TODO
            }
            FunctionRef::SAVEKILL => {
                self.run_cmd_from_functionref(FunctionRef::SAVE, arguments.clone());
                self.run_cmd_from_functionref(FunctionRef::KILL, arguments.clone());
            }
            FunctionRef::TOGGLE_CONVERTER => {
                if self.active_converter == ConverterRef::BIN {
                    self.active_converter = ConverterRef::HEX;
                } else if self.active_converter == ConverterRef::HEX {
                    self.active_converter = ConverterRef::BIN;
                }
                self.setup_displays();
                self.flag_refresh_full = true;
            }
            _ => {
                // Unknown
            }
        }
    }

    fn grab_register(&mut self, default_if_unset: isize) -> isize {
        let output;
        if self.register_isset {
            output = self.register;
            self.clear_register();
        } else {
            output = default_if_unset;
        }
        output
    }

    fn clear_register(&mut self) {
        self.register = 0;
        self.register_isset = false;
    }

    fn append_to_register(&mut self, new_digit: isize) {
        self.register *= 10;
        self.register += new_digit;
        self.register_isset = true;
    }

    // Convert argument string to bytes.
    fn string_to_bytes(&mut self, input_string: String) -> Result<Vec<u8>, ConverterError> {
        let mut use_converter: Option<Box<dyn Converter>> = None;

        let mut input_bytes = input_string.as_bytes().to_vec();
        if input_bytes.len() > 2 {
            if input_bytes[0] == 92 {
                match input_bytes[1] {
                    98 => { // b
                        use_converter = Some(Box::new(BinaryConverter {}));
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
}

// TODO: Consider quotes, apostrophes  and escapes
fn parse_words(input_string: String) -> Vec<String> {
    let mut output = Vec::new();

    for word in input_string.split_whitespace() {
        output.push(word.to_string());
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

        assert!(editor.insert_bytes(0, vec![65]).is_ok());
        assert_eq!(editor.active_content.as_slice(), [65]);
        assert!(editor.insert_bytes(10, vec![65]).is_err());
    }

    #[test]
    fn test_remove_bytes() {
        let mut editor = SbyteEditor::new();
        // Ok to kill for the test, we don't care about the
        // visuals at the moment
        editor.kill();
        editor.insert_bytes(0, vec![65]);


        assert!(editor.remove_bytes(0, 1).is_ok());
        assert_eq!(editor.active_content.as_slice(), []);
        assert!(editor.remove_bytes(1000, 300).is_err());
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

        let found = editor.find_all(vec![65, 66]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], 0);
        assert_eq!(found[1], 5);

        assert_eq!(editor.find_after(vec![65, 66], 2), Some(5));

    }
}
