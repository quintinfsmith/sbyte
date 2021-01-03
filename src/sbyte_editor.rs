use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::error::Error;
use std::{time, thread};
use std::sync::{Mutex, Arc};
use std::fmt;
use regex::bytes::Regex;

use wrecked::RectError;

// CommandLine struct
pub mod command_line;
pub mod flag;
pub mod viewport;
pub mod cursor;
pub mod converter;
pub mod tests;

use converter::{HumanConverter, BinaryConverter, HexConverter, Converter, ConverterRef, ConverterError, DecConverter};
use viewport::ViewPort;
use cursor::Cursor;
use command_line::CommandLine;

#[derive(Debug)]
pub enum SbyteError {
    PathNotSet,
    SetupFailed(RectError),
    RemapFailed(RectError),
    RowSetFailed(RectError),
    ApplyCursorFailed(RectError),
    DrawFailed(RectError),
    InvalidRegex(String),
    InvalidBinary(String),
    OutOfRange(usize, usize),
    FailedToKill,
    EmptyStack,
    NoCommandGiven,
    InvalidCommand(String)
}

impl fmt::Display for SbyteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for SbyteError {}

pub struct BackEnd {
    // Flags for tick() to know when to arrange/edit rects
    user_msg: Option<String>,
    user_error_msg: Option<String>,


    flag_loading: bool,

    //Editor
    clipboard: Vec<u8>,
    active_content: Vec<u8>,
    active_file_path: Option<String>,
    cursor: Cursor,
    active_converter: ConverterRef,
    undo_stack: Vec<(usize, usize, Vec<u8>)>, // Position, bytes to remove, bytes to insert
    redo_stack: Vec<(usize, usize, Vec<u8>)>, // Position, bytes to remove, bytes to insert

    // Commandable
    commandline: CommandLine,
    line_commands: HashMap<String, String>,
    register: Option<usize>,
    flag_input_context: Option<String>,
    new_input_sequences: Vec<(String, String, String)>,

    // VisualEditor
    viewport: ViewPort,

    search_history: Vec<String>
}

impl BackEnd {
    pub fn new() -> BackEnd {
        let mut output = BackEnd {
            flag_loading: false,

            user_msg: None,
            user_error_msg: None,

            clipboard: Vec::new(),
            active_content: Vec::new(),
            active_file_path: None,
            cursor: Cursor::new(),
            active_converter: ConverterRef::HEX,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            register: None,
            flag_input_context: None,
            new_input_sequences: Vec::new(),

            viewport: ViewPort::new(1, 1),

            line_commands: HashMap::new(),
            commandline: CommandLine::new(),

            search_history: Vec::new()
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

        output
    }

    pub fn increment_byte(&mut self, offset: usize) -> Result<(), SbyteError> {
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
            Err(SbyteError::OutOfRange(offset, self.active_content.len()))
        }
    }

    pub fn decrement_byte(&mut self, offset: usize) -> Result<(), SbyteError> {
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
            Err(SbyteError::OutOfRange(offset, self.active_content.len()))
        }
    }

    // ONLY to be used in insert_bytes and overwrite_bytes. nowhere else.
    fn _insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) {
        if offset < self.active_content.len() {
            for (i, new_byte) in new_bytes.iter().enumerate() {
                self.active_content.insert(offset + i, *new_byte);
            }
        } else if offset == self.active_content.len() {
            for new_byte in new_bytes.iter() {
                self.active_content.push(*new_byte);
            }
        } else {
            #[cfg(debug_assertions)]
            {
                //TODO Debug error log
                //logg(Err(SbyteError::OutOfRange(offset, self.active_content.len())));
            }
        }

    }

    // ONLY to be  used by remove_bytes and overwrite_bytes functions, nowhere else.
    fn _remove_bytes(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let output;
        if offset < self.active_content.len() {
            let mut removed_bytes = Vec::new();
            let adj_length = min(self.active_content.len() - offset, length);
            for _ in 0..adj_length {
                removed_bytes.push(self.active_content.remove(offset));
            }
            output = removed_bytes;
        } else {
            output = vec![];

            #[cfg(debug_assertions)]
            {
                //TODO Debug error log
                //logg(Err(SbyteError::OutOfRange(offset, self.active_content.len())));
            }
        }

        output
    }

    pub fn build_key_map() -> HashMap<&'static str, &'static str> {
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

    pub fn set_user_error_msg(&mut self, msg: &str) {
        self.user_error_msg = Some(msg.to_string());
    }

    pub fn set_user_msg(&mut self, msg: &str) {
        self.user_msg = Some(msg.to_string());
    }

    pub fn is_loading(&self) -> bool {
        self.flag_loading
    }
    pub fn add_search_history(&mut self, search_string: String) {
        self.search_history.push(search_string.clone());
    }
    pub fn undo(&mut self) -> Result<(), SbyteError> {
        let task = self.undo_stack.pop();
        match task {
            Some(_task) => {
                let redo_task = self.do_undo_or_redo(_task);
                self.redo_stack.push(redo_task);
                Ok(())
            }
            None => {
                Err(SbyteError::EmptyStack)
            }
        }
    }

    pub fn redo(&mut self) -> Result<(), SbyteError> {
        let task = self.redo_stack.pop();
        match task {
            Some(_task) => {
                let undo_task = self.do_undo_or_redo(_task);
                // NOTE: Not using self.push_to_undo_stack. don't want to clear the redo stack
                self.undo_stack.push(undo_task);
                Ok(())
            }
            None => {
                Err(SbyteError::EmptyStack)
            }
        }
    }


    fn do_undo_or_redo(&mut self, task: (usize, usize, Vec<u8>)) -> (usize, usize, Vec<u8>) {
        let (offset, bytes_to_remove, bytes_to_insert) = task;

        self.set_cursor_offset(offset);

        let mut opposite_bytes_to_insert = vec![];
        if bytes_to_remove > 0 {
            let removed_bytes = self._remove_bytes(offset, bytes_to_remove);
            opposite_bytes_to_insert = removed_bytes;
        }

        let mut opposite_bytes_to_remove = 0;
        if bytes_to_insert.len() > 0 {
            opposite_bytes_to_remove = bytes_to_insert.len();
            self._insert_bytes(offset, bytes_to_insert);
        }

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
    pub fn set_active_converter(&mut self, converter: ConverterRef) {
        self.active_converter = converter;
    }

    pub fn get_active_converter_ref(&self) -> ConverterRef {
        self.active_converter
    }

    pub fn get_active_converter(&self) -> Box<dyn Converter> {
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

    fn replace(&mut self, search_for: &str, replace_with: Vec<u8>) -> Result<(), SbyteError> {
        let mut matches = self.find_all(&search_for)?;
        // replace in reverse order
        matches.reverse();

        for (start, end) in matches.iter() {
            for j in *start..*end {
                self.active_content.remove(*start);
            }

            for (j, new_byte) in replace_with.iter().enumerate() {
                self.active_content.insert(*start + j, *new_byte);
            }
        }

        Ok(())
    }

    pub fn make_selection(&mut self, offset: usize, length: usize) {
        self.set_cursor_offset(offset);
        self.set_cursor_length(length as isize);
    }

    pub fn copy_to_clipboard(&mut self, bytes_to_copy: Vec<u8>) {
        self.clipboard = Vec::new();
        for b in bytes_to_copy.iter() {
            self.clipboard.push(*b);
        }
    }

    pub fn get_clipboard(&mut self) -> Vec<u8> {
        self.clipboard.clone()
    }

    pub fn copy_selection(&mut self) {
        let selected_bytes = self.get_selected();
        self.copy_to_clipboard(selected_bytes);
    }

    pub fn load_file(&mut self, file_path: &str) -> std::io::Result<()> {
        self.flag_loading = true;
        self.active_content = Vec::new();

        self.set_file_path(file_path);
        match File::open(file_path) {
            Ok(mut file) => {
                let file_length = match file.metadata() {
                    Ok(metadata) => {
                        metadata.len()
                    }
                    Err(_) => { // TODO: Handle different error types
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

        self.flag_loading = false;

        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        match self.active_file_path.clone() {
            Some(path) => {
                self.save_as(&path.to_string())?;
            }
            None => {
                Err(SbyteError::PathNotSet)?;
            }
        };

        Ok(())
    }

    pub fn save_as(&mut self, path: &str) -> std::io::Result<()> {
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

    pub fn find_all(&self, search_for: &str) -> Result<Vec<(usize, usize)>, SbyteError> {
        let mut modded_string: bool = false;
        let mut working_search = search_for.to_string();


        { // Look for binary byte definitions (\b) and translate them to \x
            let hexchars = vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F"];
            match Regex::new("\\\\b.{0,8}") {
                Ok(patt) => {
                    let mut hits = vec![];
                    for hit in patt.find_iter(search_for.to_string().as_bytes()) {
                        if (hit.end() - hit.start() == 2) {
                            Err(SbyteError::InvalidBinary(search_for[hit.start() .. hit.end()].to_string()))?;
                        } else {
                            hits.push((hit.start(), hit.end()));
                        }
                    }
                    hits.sort();

                    for hit in hits.iter().rev() {
                        let mut binnum: usize = 0;
                        let mut wildcard_indeces = vec![];
                        let length = (hit.1 - hit.0) - 2;
                        for (i, c) in working_search[hit.0 + 2..hit.1].chars().enumerate() {
                            binnum *= 2;
                            match c {
                                '1' => {
                                    binnum += 1;
                                }
                                '0' => {}
                                '.' => {
                                    wildcard_indeces.push(length - 1 - i);
                                }
                                _ => {
                                    Err(SbyteError::InvalidBinary(working_search[hit.0 .. hit.1].to_string()))?;
                                }
                            }
                        }

                        let mut state_bits = 0;
                        let mut possible_numbers = vec![
                            vec![
                                "\\x".to_string(),
                                hexchars[binnum / 16].to_string(),
                                hexchars[binnum % 16].to_string()
                            ].join("")
                        ];

                        for i in 0 .. 2_u32.pow(wildcard_indeces.len() as u32) as usize {
                            state_bits += 1;
                            let mut testn = binnum + 0;
                            for j in 0 .. wildcard_indeces.len() {
                                if state_bits & (2_u32.pow(j as u32) as usize) != 0 {
                                    testn += 2_u32.pow((wildcard_indeces[j] as u32)) as usize;
                                }
                            }

                            possible_numbers.push(
                                vec![
                                    "\\x".to_string(),
                                    hexchars[testn / 16].to_string(),
                                    hexchars[testn % 16].to_string()
                                ].join("")
                            );
                        }

                        working_search = vec![
                            working_search[0..hit.0].to_string(),
                            "[".to_string(),
                            possible_numbers.join(""),
                            "]".to_string(),
                            working_search[hit.1 ..].to_string()
                        ].join("");
                    }
                }
                Err(e) => { }
            }
        }

        { // Look for wildcard in byte definitions, eg "\x.0" or "\x9."
            let hexchars = "012345789ABCDEF";
            match Regex::new("\\\\x[0-9a-fA-f]\\.") {
                Ok(patt) => {
                    let mut hits = vec![];
                    for hit in patt.find_iter(search_for.to_string().as_bytes()) {
                        hits.push(hit.start());
                    }
                    hits.sort();
                    for hit in hits.iter().rev() {
                        let consistent_chunk = working_search[*hit..*hit + 3].to_string();
                        let mut option_chunks = vec![];
                        for hchar in hexchars.chars() {
                            option_chunks.push(
                                vec![consistent_chunk.clone(), hchar.to_string()].join("").to_string()
                            )
                        }

                        working_search = vec![
                            working_search[0..*hit].to_string(),
                            "[".to_string(),
                            option_chunks.join("").to_string(),
                            "]".to_string(),
                            working_search[*hit + 4..].to_string()
                        ].join("");

                    }
                }
                Err(e) => { }
            }

            match Regex::new("\\\\x\\.[0-9a-fA-F]") {
                Ok(patt) => {
                    let mut hits = vec![];
                    for hit in patt.find_iter(search_for.to_string().as_bytes()) {
                        hits.push(hit.start());
                    }
                    hits.sort();
                    for hit in hits.iter().rev() {
                        let consistent_chunk = working_search[*hit + 3..*hit + 4].to_string();
                        let mut option_chunks = vec![];
                        for hchar in hexchars.chars() {
                            option_chunks.push(
                                vec![ "\\x".to_string(), hchar.to_string(), consistent_chunk.clone()].join("").to_string()
                            )
                        }

                        working_search = vec![
                            working_search[0..*hit].to_string(),
                            "[".to_string(),
                            option_chunks.join("").to_string(),
                            "]".to_string(),
                            working_search[*hit + 4..].to_string()
                        ].join("");

                    }
                }
                Err(e) => { }
            }
        }

        let mut output = Vec::new();
        let working_string = format!("(?-u:{})", working_search);
        match Regex::new(&working_string) {
            Ok(patt) => {

                for hit in patt.find_iter(&self.active_content) {
                    output.push((hit.start(), hit.end()))
                }

                output.sort();
            }
            Err(e) => {
                Err(SbyteError::InvalidRegex(search_for.to_string()))?
            }
        }

        Ok(output)
    }

    pub fn find_nth_after(&self, pattern: &str, offset: usize, n: usize) -> Result<Option<(usize, usize)>, SbyteError> {
        //TODO: This could definitely be sped up.
        let matches = self.find_all(pattern)?;
        let mut match_index = 0;

        if matches.len() > 0 {
            for (i, (x, _)) in matches.iter().enumerate() {
                if *x > offset {
                    match_index = i;
                    break;
                }
            }

            match_index = (match_index + n) % matches.len();

            Ok(Some(matches[match_index]))
        } else {
            Ok(None)
        }
    }

    pub fn find_after(&self, pattern: &str, offset: usize) -> Result<Option<(usize, usize)>, SbyteError> {
        self.find_nth_after(pattern, offset, 0)
    }


    pub fn remove_bytes(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let removed_bytes = self._remove_bytes(offset, length);
        self.push_to_undo_stack(offset, 0, removed_bytes.clone());

        removed_bytes
    }


    pub fn remove_bytes_at_cursor(&mut self) -> Vec<u8> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();
        self.remove_bytes(offset, length)
    }


    pub fn insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) {
        let adj_byte_width = new_bytes.len();
        self._insert_bytes(offset, new_bytes);

        self.push_to_undo_stack(offset, adj_byte_width, vec![]);
    }

    pub fn overwrite_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) -> Vec<u8> {
        let position = self.cursor.get_offset();
        self.overwrite_bytes(position, new_bytes)
    }

    pub fn overwrite_bytes(&mut self, position: usize, new_bytes: Vec<u8>) -> Vec<u8> {
        let length = new_bytes.len();
        let removed_bytes = self._remove_bytes(position, length);

        self._insert_bytes(position, new_bytes);
        self.push_to_undo_stack(position, length, removed_bytes.clone());

        removed_bytes
    }

    pub fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) {
        let position = self.cursor.get_offset();
        self.insert_bytes(position, new_bytes);
    }

    pub fn get_selected(&mut self) -> Vec<u8> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();

        self.get_chunk(offset, length)
    }

    pub fn get_chunk(&self, offset: usize, length: usize) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        for i in min(offset, self.active_content.len()) .. min(self.active_content.len(), offset + length) {
            output.push(self.active_content[i]);
        }

        output
    }

    pub fn cursor_next_byte(&mut self) {
        let new_position = self.cursor.get_offset() + 1;
        self.set_cursor_offset(new_position);
    }

    pub fn cursor_prev_byte(&mut self) {
        if self.cursor.get_offset() != 0 {
            let new_position = self.cursor.get_offset() - 1;
            self.set_cursor_offset(new_position);
        }
    }

    pub fn cursor_increase_length(&mut self) {
        let new_length;
        if self.cursor.get_real_length() == -1 {
            new_length = 1;
        } else {
            new_length = self.cursor.get_real_length() + 1;
        }

        self.set_cursor_length(new_length);
    }

    pub fn cursor_decrease_length(&mut self) {
        let new_length;
        if self.cursor.get_real_length() == 1 {
            new_length = -1
        } else {
            new_length = self.cursor.get_real_length() - 1;
        }

        self.set_cursor_length(new_length);
    }

    pub fn set_cursor_offset(&mut self, new_offset: usize) {
        let adj_offset = min(self.active_content.len(), new_offset);
        self.cursor.set_offset(adj_offset);
        self.adjust_viewport_offset();
    }

    pub fn set_cursor_length(&mut self, new_length: isize) {
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
        self.adjust_viewport_offset();
    }

    pub fn get_display_ratio(&self) -> u8 {
        let human_converter = HumanConverter {};
        let human_string_length = human_converter.encode(vec![65]).len();

        let active_converter = self.get_active_converter();
        let active_string_length = active_converter.encode(vec![65]).len();

        ((active_string_length / human_string_length) + 1) as u8
    }

    pub fn get_cursor_offset(&self) -> usize {
        self.cursor.get_offset()
    }

    pub fn get_cursor_length(&self) -> usize {
        self.cursor.get_length()
    }

    pub fn get_active_content(&self) -> Vec<u8> {
        self.active_content.clone()
    }

    pub fn cursor_next_line(&mut self) {
        let new_offset = self.cursor.get_real_offset() + self.viewport.get_width();
        self.set_cursor_offset(new_offset);
    }

    pub fn cursor_prev_line(&mut self) {
        let viewport_width = self.viewport.get_width();
        let new_offset = self.cursor.get_real_offset() - min(self.cursor.get_real_offset(), viewport_width);
        self.set_cursor_offset(new_offset);
    }

    pub fn cursor_increase_length_by_line(&mut self) {
        let mut new_length: isize = self.cursor.get_real_length() + (self.viewport.get_width() as isize);

        if self.cursor.get_real_length() < 0 && new_length >= 0 {
            new_length += 1;
        }

        self.set_cursor_length(new_length);
    }

    pub fn cursor_decrease_length_by_line(&mut self) {
        let mut new_length: isize = self.cursor.get_real_length() - (self.viewport.get_width() as isize);
        if self.cursor.get_real_length() > 0 && new_length < 0 {
            new_length -= 1;
        }
        self.set_cursor_length(new_length);
    }

    pub fn adjust_viewport_offset(&mut self) {
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

    pub fn get_viewport_size(&self) -> (usize, usize) {
        (self.viewport.get_width(), self.viewport.get_height())
    }
    pub fn get_viewport_offset(&self) -> usize {
        self.viewport.get_offset()
    }

    pub fn set_viewport_offset(&mut self, new_offset: usize) {
        self.viewport.set_offset(new_offset);
    }

    pub fn set_viewport_size(&mut self, width: usize, height: usize) {
        self.viewport.set_size(width, height);
        // Align the viewport with the new size to maintain sanity
        let old_offset = self.viewport.get_offset();
        self.viewport.set_offset((old_offset / width) * width);
    }

    pub fn get_commandline(&self) -> Option<&CommandLine> {
        Some(&self.commandline)
    }
    pub fn get_commandline_mut(&mut self) -> Option<&mut CommandLine> {
        Some(&mut self.commandline)
    }

    pub fn get_active_file_path(&self) -> Option<&String> {
        self.active_file_path.as_ref()
    }

    pub fn get_search_history(&self) -> Vec<String> {
        self.search_history.clone()
    }

    pub fn try_command(&mut self, query: &str) -> Result<(String, Vec<String>), Box<dyn Error>> {
        let mut words = parse_words(query.to_string());
        if words.len() > 0 {
            let cmd = words.remove(0);
            let mut arguments: Vec<String> = vec![];

            for word in words.iter() {
                arguments.push(word.clone());
            }

            match self.line_commands.get(&cmd) {
                Some(_funcref) => {
                    Ok((_funcref.to_string(), arguments.clone()))
                }
                None => {
                    Err(Box::new(SbyteError::InvalidCommand(query.to_string())))
                }
            }
        } else {
            Err(Box::new(SbyteError::NoCommandGiven))
        }
    }

    fn assign_line_command(&mut self, command_string: &str, function: &str) {
        self.line_commands.insert(command_string.to_string(), function.to_string());
    }

    pub fn unset_user_msg(&mut self) {
        self.user_msg = None;
    }
    pub fn unset_user_error_msg(&mut self) {
        self.user_error_msg = None;
    }

    pub fn get_user_msg(&self) -> Option<&String> {
        self.user_msg.as_ref()
    }
    pub fn get_user_error_msg(&self) -> Option<&String> {
        self.user_error_msg.as_ref()
    }
}

pub fn parse_words(input_string: String) -> Vec<String> {
    let mut output = Vec::new();

    let mut delimiters = HashMap::new();
    delimiters.insert(' ', ' ');
    delimiters.insert('"', '"');
    delimiters.insert('\'', '\'');

    let mut working_word: String = "".to_string();
    let mut opener: Option<char> = None;
    let mut is_escaped = false;
    for c in input_string.chars() {
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

pub fn string_to_integer(input_string: &str) -> Result<usize, ConverterError> {
    let mut use_converter: Option<Box<dyn Converter>> = None;

    let input_bytes = input_string.to_string().as_bytes().to_vec();
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
            for character in input_string.chars() {
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

// Convert argument string to bytes.
pub fn string_to_bytes(input_string: String) -> Result<Vec<u8>, ConverterError> {
    let mut use_converter: Option<Box<dyn Converter>> = None;

    let input_bytes = input_string.as_bytes().to_vec();
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
