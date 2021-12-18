use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fs::File;
use std::io::{Write, Read};
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};
use std::convert::From;
use regex::bytes::Regex;
use std::num::ParseIntError;

use wrecked::WreckedError;

pub mod viewport;
pub mod cursor;
pub mod formatter;
pub mod tests;
pub mod content;

use formatter::{BinaryFormatter, HexFormatter, Formatter, FormatterRef, DecFormatter, FormatterError};
use viewport::ViewPort;
use cursor::Cursor;
use content::{Content, ContentError, BitMask};



#[derive(Debug, Eq, PartialEq)]
pub enum SbyteError {
    PathNotSet,
    SetupFailed(WreckedError),
    RemapFailed(WreckedError),
    RowSetFailed(WreckedError),
    ApplyCursorFailed(WreckedError),
    DrawFailed(WreckedError),
    InvalidRegex(String),
    InvalidBinary(String),
    InvalidHexidecimal(String),
    InvalidDecimal(String),
    OutOfBounds(usize, usize),
    FailedToKill,
    EmptyStack,
    NoCommandGiven,
    ReadFail,
    // TODO: Move InvalidCommand
    InvalidCommand(String),
    ConversionFailed,
    InvalidDigit(FormatterRef),
    InvalidRadix(u8),
    BufferEmpty,
    KillSignal
}

impl fmt::Display for SbyteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for SbyteError {}

impl From<ContentError> for SbyteError {
    fn from(err: ContentError) -> Self {
        match err {
            ContentError::OutOfBounds(offset, length) => {
                SbyteError::OutOfBounds(offset, length)
            }
            ContentError::InvalidDigit(digit, radix) => {
                match radix {
                    16 => SbyteError::InvalidDigit(FormatterRef::HEX),
                    10 => SbyteError::InvalidDigit(FormatterRef::DEC),
                    2 =>  SbyteError::InvalidDigit(FormatterRef::BIN),
                    _ => SbyteError::InvalidRadix(radix)
                }
            }
        }
    }
}

impl From<FormatterError> for SbyteError {
    fn from(err: FormatterError) -> Self {
        match err {
            FormatterError::InvalidDigit(conv) => {
                SbyteError::InvalidDigit(conv)
            }
        }
    }
}

pub struct Editor {
    user_msg: Option<String>,
    user_error_msg: Option<String>,

    flag_loading: bool,

    //Editor
    clipboard: Vec<u8>,
    active_content: Content,
    active_file_path: Option<String>,
    cursor: Cursor,
    subcursor: Cursor,
    active_formatter: FormatterRef,
    undo_stack: Vec<(usize, usize, Vec<u8>, Instant)>, // Position, bytes to remove, bytes to insert
    redo_stack: Vec<(usize, usize, Vec<u8>, Instant)>, // Position, bytes to remove, bytes to insert

    // VisualEditor
    viewport: ViewPort,

    search_history: Vec<String>,
    change_flags: HashMap<String, bool>,
    changed_offsets: HashSet<(usize, bool)>,

    _active_display_ratio: u8
}

impl Editor {
    pub fn new() -> Editor {
        let mut output = Editor {
            flag_loading: false,

            user_msg: None,
            user_error_msg: None,

            clipboard: Vec::new(),
            active_content: Content::new(),
            active_file_path: None,
            cursor: Cursor::new(),
            subcursor: Cursor::new(),
            active_formatter: FormatterRef::HEX,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),

            viewport: ViewPort::new(1, 1),


            search_history: Vec::new(),
            change_flags: HashMap::new(),
            changed_offsets: HashSet::new(),

            _active_display_ratio: 3
        };

        output.set_cursor_length(1);
        output.set_cursor_offset(0);
        output.set_subcursor_length();

        output
    }

    pub fn len(&self) -> usize {
        self.active_content.len()
    }

    pub fn increment_byte(&mut self, offset: usize, word_size: usize) -> Result<(), SbyteError> {
        match self.active_content.increment_byte(offset, word_size) {
            Ok(undo_bytes) => {
                let undo_len = undo_bytes.len();
                let undo_offset = offset + 1 - undo_len;
                self.push_to_undo_stack(undo_offset, undo_len, undo_bytes);
                Ok(())
            }
            Err(_) => {
                Err(SbyteError::OutOfBounds(offset, self.active_content.len()))
            }
        }
    }

    pub fn decrement_byte(&mut self, offset: usize, word_size: usize) -> Result<(), SbyteError> {
        match self.active_content.decrement_byte(offset, word_size) {
            Ok(undo_bytes) => {
                let undo_len = undo_bytes.len();
                let undo_offset = offset + 1 - undo_len;
                self.push_to_undo_stack(undo_offset, undo_len, undo_bytes);
                Ok(())
            }
            Err(_) => {
                Err(SbyteError::OutOfBounds(offset, self.active_content.len()))
            }
        }
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

    pub fn undo(&mut self) -> Result<usize, SbyteError> {
        let mut tasks_undone = 0;
        let threshold = Duration::from_nanos(50_000_000);
        let mut latest_instant: Option<Instant> = None;
        loop {
            let task_option = self.undo_stack.pop();
            match task_option {
                Some(task) => {
                    if match latest_instant {
                        Some(then) => { (then - task.3) <= threshold }
                        None => { true }
                    } {
                        latest_instant = Some(task.3.clone());
                        let redo_task = self.do_undo_or_redo(task)?;
                        self.redo_stack.push(redo_task);
                        tasks_undone += 1;
                    } else {
                        self.undo_stack.push(task);
                        break;
                    }
                }
                None => {
                    break;
                }
            }
        }

        if tasks_undone > 0 {
            Ok(tasks_undone)
        } else {
            Err(SbyteError::EmptyStack)
        }
    }

    pub fn redo(&mut self) -> Result<usize, SbyteError> {
        let mut tasks_redone = 0;
        let threshold = Duration::from_nanos(50_000_000);
        let mut latest_instant: Option<Instant> = None;
        loop {
            let task_option = self.redo_stack.pop();
            match task_option {
                Some(task) => {
                    if match latest_instant {
                        Some(then) => { (task.3 - then) <= threshold }
                        None => { true }
                    } {
                        latest_instant = Some(task.3.clone());
                        let redo_task = self.do_undo_or_redo(task)?;
                        self.undo_stack.push(redo_task);
                        tasks_redone += 1;
                    } else {
                        self.undo_stack.push(task);
                        break;
                    }
                }
                None => {
                    break;
                }
            }
        }

        if tasks_redone > 0 {
            Ok(tasks_redone)
        } else {
            Err(SbyteError::EmptyStack)
        }
    }

    fn do_undo_or_redo(&mut self, task: (usize, usize, Vec<u8>, Instant)) -> Result<(usize, usize, Vec<u8>, Instant), SbyteError> {
        let (offset, bytes_to_remove, bytes_to_insert, timestamp) = task;
        self.set_cursor_length(1);
        self.set_cursor_offset(offset);

        let mut opposite_bytes_to_insert = vec![];
        if bytes_to_remove > 0 {
            let removed_bytes = self.active_content.remove_bytes(offset, bytes_to_remove);
            opposite_bytes_to_insert = removed_bytes;
        }

        let mut opposite_bytes_to_remove = 0;
        if bytes_to_insert.len() > 0 {
            opposite_bytes_to_remove = bytes_to_insert.len();
            self.active_content.insert_bytes(offset, &bytes_to_insert)?;
        }

        self.changed_offsets.insert((offset, opposite_bytes_to_remove != opposite_bytes_to_insert.len()));

        Ok((offset, opposite_bytes_to_remove, opposite_bytes_to_insert, timestamp))
    }

    fn push_to_undo_stack(&mut self, offset: usize, bytes_to_remove: usize, bytes_to_insert: Vec<u8>) {

        self.redo_stack.drain(..);
        let is_insert = bytes_to_remove == 0 && bytes_to_insert.len() > 0;
        let is_remove = bytes_to_remove > 0 && bytes_to_insert.len() == 0;
        let is_overwrite = !is_insert && !is_remove;


        let mut was_merged = false;
        match self.undo_stack.last_mut() {
            Some((next_offset, next_bytes_to_remove, next_bytes_to_insert, prev_timestamp)) => {
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

                if was_merged {
                    *prev_timestamp = Instant::now();
                }
            }
            None => ()
        }

        self.changed_offsets.insert((offset, bytes_to_remove != bytes_to_insert.len()));

        if !was_merged {
            self.undo_stack.push((offset, bytes_to_remove, bytes_to_insert, Instant::now()));
        }

    }

    pub fn set_active_formatter(&mut self, formatter: FormatterRef) {
        self.active_formatter = formatter;
        match self.get_active_formatter().radix() {
            radix => {
                let l = 256.0_f64.log(radix as f64);
                self._active_display_ratio = (l.ceil() as u8) + 1
            }
        }
        self.set_subcursor_length();
    }

    pub fn get_active_formatter_ref(&self) -> FormatterRef {
        self.active_formatter
    }

    pub fn get_active_formatter(&self) -> Box<dyn Formatter> {
        match self.active_formatter {
            FormatterRef::HEX => {
                Box::new(HexFormatter {})
            }
            FormatterRef::BIN => {
                Box::new(BinaryFormatter {})
            }
            FormatterRef::DEC => {
                Box::new(DecFormatter {})
            }
        }
    }

    pub fn toggle_formatter(&mut self) {
        let new_formatter = match self.get_active_formatter_ref() {
            FormatterRef::BIN => {
                FormatterRef::HEX
            }
            FormatterRef::HEX => {
                FormatterRef::DEC
            }
            FormatterRef::DEC => {
                FormatterRef::BIN
            }
        };
        self.set_active_formatter(new_formatter);
    }

    pub fn replace(&mut self, search_for: &str, replace_with: &[u8]) -> Result<Vec<usize>, SbyteError> {
        let mut matches = self.find_all(&search_for)?;
        // replace in reverse order
        matches.sort();
        matches.reverse();

        let mut removed_bytes: Vec<u8>;
        let mut hit_positions: Vec<usize> = Vec::new();
        for (start, end) in matches.iter() {
            hit_positions.push(*start);
            removed_bytes = self.active_content.remove_bytes(*start, *end - *start);
            self.active_content.insert_bytes(*start, replace_with)?;
            self.push_to_undo_stack(*start, replace_with.len(), removed_bytes.clone());
        }

        Ok(hit_positions)
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

    pub fn get_clipboard(&self) -> Vec<u8> {
        self.clipboard.clone()
    }

    pub fn copy_selection(&mut self) {
        let selected_bytes = self.get_selected();
        self.copy_to_clipboard(selected_bytes);
    }

    pub fn load_file(&mut self, file_path: &str) -> std::io::Result<()> {
        self.flag_loading = true;
        self.active_content = Content::new();

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
        let mut working_search = search_for.to_string();

        { // Look for binary byte definitions (\b) and translate them to \x
            let hexchars = vec!["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F"];
            match Regex::new("\\\\b.{0,8}") {
                Ok(patt) => {
                    let mut hits = vec![];
                    for hit in patt.find_iter(search_for.to_string().as_bytes()) {
                        if hit.end() - hit.start() == 2 {
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

                        for _ in 0 .. 2_u32.pow(wildcard_indeces.len() as u32) as usize {
                            state_bits += 1;
                            let mut testn = binnum + 0;
                            for j in 0 .. wildcard_indeces.len() {
                                if state_bits & (2_u32.pow(j as u32) as usize) != 0 {
                                    testn += 2_u32.pow(wildcard_indeces[j] as u32) as usize;
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
                Err(_e) => { }
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
                Err(_e) => { }
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
                Err(_e) => { }
            }
        }

        match self.active_content.find_all(&working_search) {
            Ok(output) => {
                Ok(output)
            }
            Err(_) => {
                Err(SbyteError::InvalidRegex(search_for.to_string()))
            }
        }
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

    pub fn find_nth_before(&self, pattern: &str, offset: usize, n: usize) -> Result<Option<(usize, usize)>, SbyteError> {
        //TODO: This could definitely be sped up.
        let matches = self.find_all(pattern)?;

        if matches.len() > 0 {
            let mut match_index = matches.len() - 1;

            for (i, (x, _)) in matches.iter().enumerate() {
                if *x >= offset {
                    break;
                } else {
                    match_index = i;
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

    pub fn find_before(&self, pattern: &str, offset: usize) -> Result<Option<(usize, usize)>, SbyteError> {
        self.find_nth_before(pattern, offset, 0)
    }

    pub fn remove_bytes(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let removed_bytes = self.active_content.remove_bytes(offset, length);
        self.push_to_undo_stack(offset, 0, removed_bytes.clone());

        removed_bytes
    }

    pub fn remove_bytes_at_cursor(&mut self) -> Vec<u8> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();
        self.remove_bytes(offset, length)
    }

    pub fn insert_bytes(&mut self, offset: usize, new_bytes: &[u8]) -> Result<(), SbyteError> {
        let adj_byte_width = new_bytes.len();
        self.active_content.insert_bytes(offset, new_bytes)?;

        self.push_to_undo_stack(offset, adj_byte_width, vec![]);
        Ok(())
    }

    pub fn replace_digit(&mut self, digit: char) -> Result<(), SbyteError> {
        let formatter = self.get_active_formatter();
        let radix = formatter.radix();
        let offset = self.get_cursor_offset() + (self.get_subcursor_offset() / self.get_subcursor_length());

        let subcursor_real_position = self.subcursor.get_length() - 1 - (self.subcursor.get_offset() % self.subcursor.get_length());
        match digit.to_digit(radix as u32) {
            Some(value) => {
                let old_byte = self.active_content.replace_digit(offset, subcursor_real_position as u8, value as u8, radix as u8)?;
                self.push_to_undo_stack(offset, 1, vec![old_byte]);
                Ok(())
            }
            None => {
                Err(SbyteError::InvalidDigit(self.get_active_formatter_ref()))
            }
        }
    }

    pub fn overwrite_bytes(&mut self, position: usize, new_bytes: &[u8]) -> Result<Vec<u8>, SbyteError> {
        let length = new_bytes.len();
        let removed_bytes = self.active_content.remove_bytes(position, length);

        self.active_content.insert_bytes(position, new_bytes)?;
        self.push_to_undo_stack(position, length, removed_bytes.clone());

        Ok(removed_bytes)
    }

    pub fn insert_bytes_at_cursor(&mut self, new_bytes: &[u8]) -> Result<(), SbyteError> {
        let position = self.cursor.get_offset();
        self.insert_bytes(position, new_bytes)
    }

    pub fn overwrite_bytes_at_cursor(&mut self, new_bytes: &[u8]) -> Result<Vec<u8>, SbyteError> {
        let position = self.cursor.get_offset();
        self.overwrite_bytes(position, new_bytes)
    }

    pub fn get_selected(&mut self) -> Vec<u8> {
        let offset = self.cursor.get_offset();
        let length = self.cursor.get_length();

        self.get_chunk(offset, length)
    }

    pub fn get_selected_as_big_endian(&mut self) -> usize {
        let mut value = 0 as usize;
        for n in self.get_selected().iter() {
            value *= 256;
            value += *n as usize;
        }
        value
    }

    pub fn get_selected_as_little_endian(&mut self) -> usize {
        let mut value = 0 as usize;
        for n in self.get_selected().iter().rev() {
            value *= 256;
            value += *n as usize;
        }
        value
    }

    pub fn get_chunk(&self, offset: usize, length: usize) -> Vec<u8> {
        self.active_content.get_chunk(offset, length)
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

    pub fn subcursor_next_digit(&mut self) {
        let new_position = self.subcursor.get_offset() + 1;
        self.set_subcursor_offset(new_position);
    }

    pub fn subcursor_prev_digit(&mut self) {
        if self.subcursor.get_offset() != 0 {
            let new_position = self.subcursor.get_offset() - 1;
            self.set_subcursor_offset(new_position);
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

    pub fn set_cursor_offset(&mut self, new_offset: usize) -> Result<(), SbyteError> {
        if new_offset <= self.active_content.len() {
            self.cursor.set_offset(new_offset);
            self.subcursor.set_offset(0);
            self.adjust_viewport_offset();
            Ok(())
        } else {
            Err(SbyteError::OutOfBounds(new_offset, self.active_content.len()))
        }
    }

    pub fn set_cursor_length(&mut self, new_length: isize) {
        if self.cursor.get_real_offset() == self.active_content.len() && new_length > 0 {
            self.cursor.set_length(1);
        } else if new_length < 0 {
            self.cursor.set_length(max(new_length, 0 - self.cursor.get_real_offset() as isize));
        } else if new_length == 0 {
        } else {
            let adj_length = min(new_length as usize, self.active_content.len() - self.cursor.get_real_offset()) as isize;
            self.cursor.set_length(adj_length);
        }

        self.set_subcursor_length();
        self.subcursor.set_offset(0);

        self.adjust_viewport_offset();
    }

    pub fn get_display_ratio(&self) -> u8 {
        self._active_display_ratio

    }

    pub fn get_cursor_offset(&self) -> usize {
        self.cursor.get_offset()
    }

    pub fn get_cursor_real_offset(&self) -> usize {
        self.cursor.get_real_offset()
    }

    pub fn get_cursor_length(&self) -> usize {
        self.cursor.get_length()
    }

    pub fn get_cursor_real_length(&self) -> isize {
        self.cursor.get_real_length()
    }

    pub fn get_subcursor_offset(&self) -> usize {
        self.subcursor.get_offset()
    }

    pub fn get_subcursor_length(&self) -> usize {
        self.subcursor.get_length()
    }

    pub fn set_subcursor_length(&mut self) {
        self.subcursor.set_length((self._active_display_ratio - 1) as isize);
        self.set_subcursor_offset(0);
    }

    pub fn set_subcursor_offset(&mut self, new_offset: usize) {
        let modlength = self.subcursor.get_length() * self.cursor.get_length();
        self.subcursor.set_offset(new_offset % modlength);
    }

    pub fn get_active_content(&self) -> &[u8] {
        self.active_content.as_slice()
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

        self.set_viewport_offset(adj_viewport_offset);
    }

    pub fn get_viewport_size(&self) -> (usize, usize) {
        (self.viewport.get_width(), self.viewport.get_height())
    }
    pub fn get_viewport_offset(&self) -> usize {
        self.viewport.get_offset()
    }

    pub fn set_viewport_width(&mut self, new_width: usize) {
        let height = self.viewport.get_height();
        self.set_viewport_size(new_width, height);
    }

    pub fn set_viewport_offset(&mut self, new_offset: usize) {
        self.viewport.set_offset(new_offset);
    }

    pub fn set_viewport_size(&mut self, width: usize, height: usize) {
        self.viewport.set_size(width, height);
        // Align the viewport with the new size to maintain sanity
        let old_offset = self.viewport.get_offset();
        self.set_viewport_offset((old_offset / width) * width);
    }

    pub fn fetch_changed_offsets(&mut self) -> HashSet<(usize, bool)> {
        self.changed_offsets.drain().collect()
    }

    pub fn get_active_file_path(&self) -> Option<&String> {
        self.active_file_path.as_ref()
    }

    pub fn get_search_history(&self) -> Vec<String> {
        self.search_history.clone()
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

    fn apply_mask(&mut self, operation: BitMask, mask: &[u8]) -> Result<(), SbyteError> {
        let mut new_mask = Vec::new();
        let mask_len = mask.len();
        let cursor_len = self.get_cursor_length();
        if cursor_len != 1 {
            for i in 0 .. cursor_len {
                new_mask.push(mask[i % mask_len]);
            }
        } else {
            new_mask = mask.to_vec();
        }

        let offset = self.get_cursor_offset();
        let old_bytes = self.active_content.apply_mask(offset, &new_mask, operation)?;
        self.push_to_undo_stack(offset, new_mask.len(), old_bytes);

        Ok(())
    }

    pub fn apply_or_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.apply_mask(BitMask::Or, mask)
    }
    pub fn apply_xor_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.apply_mask(BitMask::Xor, mask)
    }
    pub fn apply_nor_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.apply_mask(BitMask::Nor, mask)
    }
    pub fn apply_and_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.apply_mask(BitMask::And, mask)
    }
    pub fn apply_nand_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.apply_mask(BitMask::Nand, mask)
    }

    pub fn bitwise_not(&mut self) -> Result<(), SbyteError> {
        let mut mask = Vec::new();
        for _ in 0 .. self.get_cursor_length() {
            mask.push(0xFF);
        }
        self.apply_mask(BitMask::Xor, &mask)
    }
}

/// Takes strings input within the program and parses the words.
pub fn parse_words(input_string: &str) -> Vec<String> {
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
                    } else {
                        match delimiters.get(&c) {
                            Some(test_opener) => {
                                if *test_opener == o_c {
                                    opener = None;
                                    if working_word.len() > 0 {
                                        output.push(working_word.clone());
                                    }
                                    working_word = "".to_string();
                                } else {
                                    working_word.push(c);
                                }
                            }
                            None => {
                                working_word.push(c);
                            }
                        }
                    }
                } else {
                    match c {
                        ' ' | '\'' | '"' => { }
                        _ => {
                            working_word.push('\\');
                        }
                    }
                    working_word.push(c);
                    is_escaped = false;
                }
            }
            None => {
                if is_escaped {
                    match c {
                        ' ' | '\'' | '"' => { }
                        _ => {
                            working_word.push('\\');
                        }
                    }
                    opener = Some(' ');
                    working_word.push(c);
                    is_escaped = false;
                } else {
                    if c == '\\' {
                        is_escaped = true;
                    } else if c != ' ' {
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
    }
    if working_word.len() > 0 {
        output.push(working_word.clone());
    }

    output
}

/// Take number string provided in the editor and convert it to integer
pub fn string_to_integer(input_string: &str) -> Result<usize, ParseIntError> {
    let input_bytes = input_string.to_string().as_bytes().to_vec();
    let mut radix = 10;
    let mut start = 0;
    if input_bytes.len() > 2 {
        if input_bytes[0] == 92 {
            start = 2;
            match input_bytes[1] {
                98 => { // b
                    radix = 2;
                }
                120 => { // x
                    radix = 16;
                }
                _ => { }
            }
        }
    }

    usize::from_str_radix(&input_string[start..], radix)
}

// Convert argument string to bytes.
pub fn string_to_bytes(input_string: &str) -> Result<Vec<u8>, SbyteError> {
    let input_bytes = input_string.as_bytes().to_vec();
    let mut radix = 0;
    if input_bytes.len() > 2 {
        if input_bytes[0] == 92 {
            match input_bytes[1] {
                98 => { // b
                    radix = 2;
                }
                100 => { // d
                    radix = 10;
                }
                120 => { // x
                    radix = 16;
                }
                _ => { }
            }
        }
    }

    let result;
    if radix != 0 {
        match usize::from_str_radix(&input_string[2..], radix) {
            Ok(0) => {
                result = Ok(vec![0]);
            }
            Ok(mut number) => {
                let mut bytes: Vec<u8> = vec![];
                while number > 0 {
                    bytes.push((number % 256) as u8);
                    number /= 256;
                }
                bytes.reverse();
                result = Ok(bytes);
            }
            Err(e) => {
                result = Err(e);
            }
        }
    } else {
        result = Ok(input_string.as_bytes().to_vec());
    }

    match result {
        Ok(output) => {
            Ok(output)
        }
        Err(_e) => { // FIXME: too generic
            Err(match radix {
                16 => { SbyteError::InvalidHexidecimal(input_string.to_string()) }
                2 => { SbyteError::InvalidBinary(input_string.to_string()) }
                _ => { SbyteError::InvalidDecimal(input_string.to_string()) }
            })
        }
    }
}
