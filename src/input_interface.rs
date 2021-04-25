use std::cmp::{min, max};
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::sync::{Mutex, Arc};

pub mod tests;

//TODO Move string_to_integer
use super::sbyte_editor::{BackEnd, SbyteError, string_to_integer, string_to_bytes};
use super::sbyte_editor::converter::*;
use super::console_displayer::FrontEnd;

use std::{time, thread};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct InputNode {
    node_map: HashMap<u8, usize>,
    hook: Option<(String, Vec<String>)>
}

impl InputNode {
    pub fn new() -> InputNode {
        InputNode {
            node_map: HashMap::new(),
            hook: None
        }
    }

    pub fn get_next(&self, byte: u8) -> Option<usize> {
        match self.node_map.get(&byte) {
            Some(index) => {
                Some(*index)
            }
            None => {
                None
            }
        }
    }

    pub fn set_hook(&mut self, hook: &str, args: &[&str]) {
        let mut argsvec = Vec::new();
        for arg in args.iter() {
            argsvec.push(arg.to_string());
        }

        self.hook = Some((hook.to_string(), argsvec));
    }

    pub fn get_hook(&self) -> Option<(String, Vec<String>)> {
        match &self.hook {
            Some((funcref, argsvec)) => {
                Some((funcref.clone(), argsvec.clone()))
            }
            None => {
                None
            }
        }
    }

    pub fn link_byte(&mut self, byte: u8, node_id: usize) {
        self.node_map.insert(byte, node_id);
    }
}

pub struct Inputter {
    context: String,
    killed: bool,
    input_nodes: Vec<InputNode>,
    active_node: usize,
    mode_roots: HashMap<String, usize>,
}

impl Inputter {
    pub fn new() -> Inputter {
        let mut output = Inputter {
            context: "".to_string(),
            killed: false,
            input_nodes: Vec::new(),
            active_node: 0,
            mode_roots: HashMap::new(),
        };
        output.set_context("DEFAULT");

        output
    }

    fn kill(&mut self) {
        self.killed = true;
    }

    fn get_context_root(&mut self) -> usize {
        let context = self.context.to_string();
        self.get_mode_root(&context)
    }

    fn get_mode_root(&mut self, mode: &str) -> usize {
        if ! self.mode_roots.contains_key(mode) {
            self.mode_roots.insert(mode.to_string(), self.input_nodes.len());
            self.input_nodes.push(InputNode::new());
        }
        *self.mode_roots.get(mode).unwrap()
    }

    pub fn go_to_next(&mut self, next_byte: u8) {
        let next = match self.input_nodes.get(self.active_node) {
            Some(input_node) => {
                input_node.get_next(next_byte)
            }
            None => {
                None
            }
        };

        match next {
            Some(node_id) => {
                self.active_node = node_id;
            }
            None => {
                self.active_node = self.get_context_root();
            }
        }
    }

    pub fn fetch_hook(&mut self) -> Option<(String, Vec<String>)> {
        let hook_result = match self.input_nodes.get(self.active_node) {
            Some(node) => {
                node.get_hook()
            }
            None => {
                None
            }
        };

        match hook_result {
            Some(hook) => {
                self.active_node = self.get_context_root();
                Some(hook)
            }
            None => {
                None
            }
        }
    }

    pub fn assign_mode_command(&mut self, mode: &str, command_vec: &[u8], hook: &str, args: &[&str]) {
        //let command_vec = command_string.to_string().as_bytes().to_vec();
        let mut current_node_index = self.get_mode_root(mode);

        for byte in command_vec.iter() {
            let mut flag_new_node = false;

            match self.input_nodes.get(current_node_index) {
                Some(node) => {
                    match node.get_next(*byte) {
                        Some(index) => {
                            current_node_index = index;
                        }
                        None => {
                            flag_new_node = true;
                        }
                    }
                }
                None => ()
            }

            if flag_new_node {
                let new_id = self.input_nodes.len();
                match self.input_nodes.get_mut(current_node_index) {
                    Some(node) => {
                        node.link_byte(*byte, new_id);
                    }
                    None => {
                        // Unreachable?
                    }
                }

                current_node_index = new_id;
                self.input_nodes.push(InputNode::new());
            }

        }

        match self.input_nodes.get_mut(current_node_index) {
            Some(node) => {
                node.set_hook(hook, args);
            }
            None => ()
        }
    }

    pub fn set_context(&mut self, new_context: &str) {
        self.context = new_context.to_string();
        self.active_node = self.get_mode_root(new_context);
    }

    pub fn is_alive(&self) -> bool {
        ! self.killed
    }
}

pub struct InputInterface {
    backend: BackEnd,
    frontend: FrontEnd,
    inputter: Arc<Mutex<Inputter>>,

    locked_viewport_width: Option<usize>,

    register: Option<usize>,

    running: bool
}

impl InputInterface {
    pub fn new(backend: BackEnd, frontend: FrontEnd) -> InputInterface {
        let mut interface = InputInterface {
            locked_viewport_width: None,
            running: false,
            inputter: Arc::new(Mutex::new(InputInterface::new_inputter())),
            register: None,

            backend,
            frontend
        };

        interface.setup_default_controls().ok().unwrap();
        interface.resize_backend_viewport();

        interface
    }

    fn setup_default_controls(&mut self) -> Result<(), SbyteError> {
        // Default Controls
        self.send_command("ASSIGN_INPUT", &["TOGGLE_CONVERTER", "EQUALS"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_DOWN", "J_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_UP", "K_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_LEFT", "H_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_RIGHT", "L_LOWER"])?;

        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_BIN", "SUBCURSOR_LEFT", "H_LOWER"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_BIN", "SUBCURSOR_RIGHT", "L_LOWER"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_DEC", "SUBCURSOR_LEFT", "H_LOWER"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_DEC", "SUBCURSOR_RIGHT", "L_LOWER"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_HEX", "SUBCURSOR_LEFT", "H_LOWER"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_HEX", "SUBCURSOR_RIGHT", "L_LOWER"])?;

        self.send_command("ASSIGN_INPUT", &["CURSOR_DOWN", "ARROW_DOWN"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_UP", "ARROW_UP"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_LEFT", "ARROW_LEFT"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_RIGHT", "ARROW_RIGHT"])?;

        self.send_command("ASSIGN_INPUT", &["CURSOR_LENGTH_DOWN", "J_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_LENGTH_UP", "K_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_LENGTH_LEFT", "H_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["CURSOR_LENGTH_RIGHT", "L_UPPER"])?;

        self.send_command("ASSIGN_INPUT", &["JUMP_TO_REGISTER", "G_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["JUMP_TO_NEXT_HIGHLIGHTED", "GREATERTHAN"])?;
        self.send_command("ASSIGN_INPUT", &["JUMP_TO_PREVIOUS_HIGHLIGHTED", "LESSTHAN"])?;
        self.send_command("ASSIGN_INPUT", &["POINTER_BE_JUMP", "R_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["POINTER_LE_JUMP", "T_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["DELETE", "X_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["YANK", "Y_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["PASTE", "P_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["UNDO", "U_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["REDO", "CTRL+R"])?;
        self.send_command("ASSIGN_INPUT", &["CLEAR_REGISTER", "ESCAPE"])?;
        self.send_command("ASSIGN_INPUT", &["INCREMENT", "PLUS"])?;
        self.send_command("ASSIGN_INPUT", &["DECREMENT", "DASH"])?;
        self.send_command("ASSIGN_INPUT", &["BACKSPACE", "BACKSPACE"])?;
        self.send_command("ASSIGN_INPUT", &["DELETE", "DELETE"])?;

        //self.send_command("ASSIGN_INPUT", &["MODE_SET_INSERT", "I_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_INSERT_ASCII", "I_LOWER"])?;
        //self.send_command("ASSIGN_INPUT", &["MODE_SET_INSERT_SPECIAL", "I_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_OVERWRITE", "O_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_OVERWRITE_ASCII", "O_UPPER"])?;
        //self.send_command("ASSIGN_INPUT", &["MODE_SET_OVERWRITE_SPECIAL", "O_UPPER"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_APPEND", "A_LOWER"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_SEARCH", "SLASH"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_CMD", "COLON"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_MASK_OR", "BAR"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_MASK_AND", "AMPERSAND"])?;
        self.send_command("ASSIGN_INPUT", &["MODE_SET_MASK_XOR", "CARET"])?;
        self.send_command("ASSIGN_INPUT", &["BITWISE_NOT", "TILDE"])?;

        self.send_command("ASSIGN_MODE_INPUT", &["INSERT_ASCII", "MODE_SET_DEFAULT", "ESCAPE"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_ASCII", "MODE_SET_DEFAULT", "ESCAPE"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_HEX", "MODE_SET_DEFAULT", "ESCAPE"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_DEC", "MODE_SET_DEFAULT", "ESCAPE"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_BIN", "MODE_SET_DEFAULT", "ESCAPE"])?;

        let key_map = InputInterface::build_key_map();
        let mut ascii_map = HashMap::new();
        for (key, value) in key_map.iter() {
            ascii_map.insert(value.to_string(), key.to_string());
        }

        for c in "01".as_bytes().iter() {
            let strrep = std::str::from_utf8(&[*c]).unwrap().to_string();
            let keycode = ascii_map.get(&strrep).unwrap();
            self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_BIN", "OVERWRITE_DIGIT", &keycode, &strrep]);
        }

        for c in "0123456789".as_bytes().iter() {
            let strrep = std::str::from_utf8(&[*c]).unwrap().to_string();
            let keycode = ascii_map.get(&strrep).unwrap();
            self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_DEC", "OVERWRITE_DIGIT", &keycode, &strrep]);
            self.send_command("ASSIGN_MODE_INPUT", &["INSERT_ASCII", "INSERT_STRING", &keycode, &strrep])?;

            self.send_command("ASSIGN_INPUT", &["APPEND_TO_REGISTER", &keycode, &strrep])?;
        }

        for c in "0123456789abcdef".as_bytes().iter() {
            let strrep = std::str::from_utf8(&[*c]).unwrap().to_string();
            let keycode = ascii_map.get(&strrep).unwrap();
            self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_HEX", "OVERWRITE_DIGIT", &keycode, &strrep]);
        }

        for i in 32 .. 127 {
            let strrep = std::str::from_utf8(&[i]).unwrap().to_string();
            let keycode = ascii_map.get(&strrep).unwrap();

            self.send_command("ASSIGN_MODE_INPUT", &["INSERT_ASCII", "INSERT_STRING", &keycode, &strrep])?;
            self.send_command("ASSIGN_MODE_INPUT", &["OVERWRITE_ASCII", "OVERWRITE_STRING", &keycode, &strrep])?;
            self.send_command("ASSIGN_MODE_INPUT", &["CMD", "APPEND_TO_CMDLINE", &keycode, &strrep])?;
        }

        self.send_command("ASSIGN_MODE_INPUT", &["CMD", "RUN_CUSTOM_COMMAND", "LINE_FEED"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["CMD", "MODE_SET_DEFAULT", "ESCAPE"])?;
        self.send_command("ASSIGN_MODE_INPUT", &["CMD", "CMDLINE_BACKSPACE", "BACKSPACE"])?;

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
        self.backend.set_user_msg(&format!("[{}]", self.register.unwrap()));
    }

    pub fn spawn_input_daemon(&mut self) -> std::thread::JoinHandle<()> {
        let inputter = self.inputter.clone();
        thread::spawn(move || {
            /////////////////////////////////
            // Rectmanager puts stdout in non-canonical mode,
            // so stdin will be char-by-char
            let stdout = io::stdout();
            let mut reader = io::stdin();
            let mut buffer: [u8; 1] = [0;1];
            stdout.lock().flush().unwrap();
            ////////////////////////////////

            let mut killed: bool = false;
            let mut retry_lock: bool = false;
            while ! killed {
                if ! retry_lock {
                    reader.read_exact(&mut buffer).unwrap();
                }

                match inputter.try_lock() {
                    Ok(ref mut mutex) => {
                        killed = !mutex.is_alive();
                        if ! killed {
                            &mutex.go_to_next(buffer[0]);
                        }
                        retry_lock = false;
                    }
                    Err(_e) => {
                        retry_lock = true;
                    }
                }
            }
        })
    }

    pub fn spawn_ctrl_c_daemon(&mut self) {
        let signal_mutex = self.inputter.clone();
        // Catch the Ctrl+C Signal
        ctrlc::set_handler(move || {
            let mut ok = false;
            while !ok {
                match signal_mutex.try_lock() {
                    Ok(ref mut mutex) => {
                        mutex.kill();
                        ok = true;
                    }
                    Err(_e) => ()
                }
            }
        }).expect("Error setting Ctrl-C handler");
    }

    pub fn new_inputter() -> Inputter {
        let mut output = Inputter::new();
        output
    }

    pub fn main(&mut self) -> Result<(), SbyteError> {
        self.spawn_ctrl_c_daemon();
        self.auto_resize();
        let mut _input_daemon = self.spawn_input_daemon();

        let fps = 59.97;
        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);

        self.running = true;
        while self.running {
            match self.frontend.tick(&mut self.backend) {
                Ok(_) => {
                    self.backend.unset_user_error_msg();
                    self.backend.unset_user_msg();
                }
                Err(boxed_error) => {
                    // To help debug ...
                    self.backend.set_user_error_msg(&format!("{:?}", boxed_error));
                }
            }

            let mut funcpair = None;
            match self.inputter.try_lock() {
                Ok(ref mut mutex) => {
                    // Kill the main loop is the input loop dies
                    if ! mutex.is_alive() {
                        self.running = false;
                    }

                    funcpair = mutex.fetch_hook();
                }
                Err(_e) => ()
            }

            match funcpair {
                Some((funcref, args)) => {
                    let mut str_args = Vec::new();
                    for arg in args.iter() {
                        str_args.push(arg.as_str());
                    }
                    self.send_command(&funcref, str_args.as_slice())?;
                }
                None => {
                    thread::sleep(delay);
                }
            }

            self.auto_resize();
        }

        // Kill input thread
        match self.inputter.try_lock() {
            Ok(ref mut mutex) => {
                mutex.kill();
            }
            Err(_e) => {}
        }

        self.frontend.kill()?;

        Ok(())
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

    fn send_command(&mut self, funcref: &str, arguments: &[&str]) -> Result<(), SbyteError> {
        match funcref {
            "ASSIGN_INPUT" => {
                let mut alt_args = vec!["DEFAULT"];
                for arg in arguments.iter() {
                    alt_args.push(*arg);
                }
                self.send_command("ASSIGN_MODE_INPUT", &alt_args)?;
            }

            "ASSIGN_MODE_INPUT" => {
                let mode_key: &str = match arguments.get(0) {
                    Some(arg) => {
                        arg
                    }
                    None => {
                 //       Err(SbyteError::InvalidCommand(arguments.join("")))?;
                        ""
                    }
                };

                let new_funcref: &str = match arguments.get(1) {
                    Some(_new_funcref) => {
                        _new_funcref
                    }
                    None => {
                        ""
                    }
                };

                let new_input_string: Vec<u8> = match arguments.get(2) {
                    Some(_new_inputs) => {
                        let key_map = InputInterface::build_key_map();
                        let mut output = Vec::new();
                        for word in _new_inputs.split(",") {
                            match key_map.get(word) {
                                Some(seq) => {
                                    for byte in seq.as_bytes().to_vec().iter() {
                                        output.push(*byte);
                                    }
                                }
                                None => () // TODO: ERROR
                            }
                        }
                        output
                    }
                    None => {
                        Vec::new()
                    }
                };

                match self.inputter.try_lock() {
                    Ok(ref mut mutex) => {
                        mutex.assign_mode_command(mode_key, &new_input_string, &new_funcref, &arguments[3..]);
                    }
                    Err(_e) => ()
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

            "SUBCURSOR_LEFT" => {
                self.ci_subcursor_left();
            }

            "SUBCURSOR_RIGHT" => {
                self.ci_subcursor_right();
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

            "REPLACE_ALL" => {
                let change_from = arguments.get(0).unwrap();
                let change_to = arguments.get(1).unwrap();

                self.ci_replace(change_from, change_to);
            }

            "JUMP_TO_PREVIOUS_HIGHLIGHTED" => {
                let n = self.grab_register(0);
                let selection = self.backend.get_selected();
                let mut string_rep = "".to_string();
                for ord in selection.iter() {
                    string_rep = format!("{}\\x{:X}{:X}", string_rep, ord >> 4, ord & 0x0F);
                }
                self.ci_jump_to_previous(Some(&string_rep), n);
            }

            "JUMP_TO_NEXT_HIGHLIGHTED" => {
                let n = self.grab_register(0);
                let selection = self.backend.get_selected();
                let mut string_rep = "".to_string();
                for ord in selection.iter() {
                    string_rep = format!("{}\\x{:X}{:X}", string_rep, ord >> 4, ord & 0x0F);
                }
                self.ci_jump_to_next(Some(&string_rep), n);
            }

            "POINTER_BE_JUMP" => {
                let new_offset = self.backend.get_selected_as_big_endian();
                self.ci_jump_to_position(new_offset);
            }

            "POINTER_LE_JUMP" => {
                let new_offset = self.backend.get_selected_as_little_endian();
                self.ci_jump_to_position(new_offset);
            }

            "JUMP_TO_REGISTER" => {
                let new_offset = max(0, self.grab_register(self.backend.len())) as usize;
                self.ci_jump_to_position(new_offset);
            }

            "JUMP_TO_NEXT" => {
                let n = self.grab_register(0);
                match arguments.get(0) {
                    Some(pattern) => {
                        self.ci_jump_to_next(Some(pattern), n);
                    }
                    None => {
                        self.ci_jump_to_next(None, n);
                    }
                }
            }

            "JUMP_TO_PREVIOUS" => {
                let n = self.grab_register(0);
                match arguments.get(0) {
                    Some(pattern) => {
                        self.ci_jump_to_previous(Some(pattern), n);
                    }
                    None => {
                        self.ci_jump_to_previous(None, n);
                    }
                }
            }

            "CMDLINE_BACKSPACE" => {
                self.backend.pop_from_commandline();
            }

            "MODE_SET_MASK_XOR" => {
                self.backend.set_commandline_register("xor ");
                self.set_context("CMD");
            }
            "MODE_SET_MASK_OR" => {
                self.backend.set_commandline_register("or ");
                self.set_context("CMD");
            }
            "MODE_SET_MASK_AND" => {
                self.backend.set_commandline_register("and ");
                self.set_context("CMD");
            }
            "BITWISE_NOT" => {
                self.ci_bitwise_not()?;
            }
            "MASK_NAND" => {
                match arguments.get(0) {
                    Some(arg) => {
                        let mask = string_to_bytes(arg)?;
                        self.ci_apply_nand_mask(&mask)?;
                    }
                    None => {
                        self.backend.set_user_error_msg("No mask provided");
                    }
                }
            }
            "MASK_NOR" => {
                match arguments.get(0) {
                    Some(arg) => {
                        let mask = string_to_bytes(arg)?;
                        self.ci_apply_nor_mask(&mask)?;
                    }
                    None => {
                        self.backend.set_user_error_msg("No mask provided");
                    }
                }
            }
            "MASK_XOR" => {
                match arguments.get(0) {
                    Some(arg) => {
                        let mask = string_to_bytes(arg)?;
                        self.ci_apply_xor_mask(&mask)?;
                    }
                    None => {
                        self.backend.set_user_error_msg("No mask provided");
                    }
                }
            }

            "MASK_OR" => {
                match arguments.get(0) {
                    Some(arg) => {
                        let mask = string_to_bytes(arg)?;
                        self.ci_apply_or_mask(&mask)?;
                    }
                    None => {
                        self.backend.set_user_error_msg("No mask provided");
                    }
                }
            }
            "MASK_AND" => {
                match arguments.get(0) {
                    Some(arg) => {
                        let mask = string_to_bytes(arg)?;
                        self.ci_apply_and_mask(&mask)?;
                    }
                    None => {
                        self.backend.set_user_error_msg("No mask provided");
                    }
                }
            }

            "YANK" => {
                self.ci_yank();
            }

            "PASTE" => {
                let repeat = self.grab_register(1);
                let to_paste = self.backend.get_clipboard();
                self.ci_insert_bytes(&to_paste, repeat)?;
            }

            "DELETE" => {
                let repeat = self.grab_register(1);
                self.ci_delete(repeat);
            }

            "BACKSPACE" => {
                let repeat = min(self.backend.get_cursor_offset(), max(1, self.grab_register(1)) as usize);
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

            "MODE_SET_INSERT_ASCII" => {
                self.clear_register();
                self.set_context("INSERT_ASCII");
                self.backend.set_user_msg("--INSERT--");
            }

            "MODE_SET_INSERT_SPECIAL" => {
                self.backend.set_commandline_register("insert ");
                self.set_context("CMD");
            }

            "MODE_SET_OVERWRITE_SPECIAL" => {
                self.backend.set_commandline_register("overwrite ");
                self.set_context("CMD");
            }

            "MODE_SET_OVERWRITE" => {
                self.clear_register();
                match self.backend.get_active_converter_ref() {
                    ConverterRef::BIN => {
                        self.set_context("OVERWRITE_BIN");
                    }
                    ConverterRef::HEX => {
                        self.set_context("OVERWRITE_HEX");
                    }
                    ConverterRef::DEC => {
                        self.set_context("OVERWRITE_DEC");
                    }
                };
                self.backend.set_user_msg("--OVERWRITE--");
            }

            "MODE_SET_OVERWRITE_ASCII" => {
                self.clear_register();
                self.set_context("OVERWRITE_ASCII");
                self.backend.set_user_msg("--OVERWRITE--");
            }

            "MODE_SET_APPEND" => {
                self.ci_cursor_right(1);
                self.send_command("MODE_SET_INSERT", arguments);
            }

            "MODE_SET_DEFAULT" => {
                self.clear_register();
                self.set_context("DEFAULT");

            }

            "MODE_SET_CMD" => {
                self.backend.clear_commandline_register();
                self.set_context("CMD");
            }

            "MODE_SET_SEARCH" => {
                self.backend.set_commandline_register("find ");
                self.set_context("CMD");
            }

            "OVERWRITE_DIGIT" => {
                for arg in arguments.iter() {
                    match arg.chars().next() {
                        Some(c) => {
                            match self.ci_replace_digit(c) {
                                Ok(_) => {}
                                Err(SbyteError::InvalidDigit(conv)) => {
                                    self.backend.set_user_error_msg(&format!("Invalid digit {}", c));
                                }
                                Err(SbyteError::InvalidRadix(radix)) => {
                                    self.backend.set_user_error_msg(&format!("Invalid radix {}", radix));
                                }
                                Err(SbyteError::OutOfBounds(_, _)) => {
                                    self.backend.set_user_error_msg("Nothing to overwrite");
                                }
                                Err(e) => {
                                    Err(e)?;
                                }
                            }
                        }
                        None => ()
                    }
                }
            }

            "INSERT_STRING" => {
                let pattern = match arguments.get(0) {
                    Some(argument) => {
                        argument
                    }
                    None => {
                        ""
                    }
                };
                let repeat = self.grab_register(1);
                self.ci_insert_string(pattern, repeat)?;
            }

            //"INSERT_RAW" => {
            //    let repeat = self.grab_register(1);
            //    let pattern = match arguments.get(0) {
            //        Some(arg) => {
            //            arg.as_bytes().to_vec()
            //        }
            //        None => {
            //            vec![]
            //        }
            //    };
            //    self.ci_insert_bytes(&pattern, repeat)?;
            //}

            "APPEND_TO_CMDLINE" => {
                match arguments.get(0) {
                    Some(argument) => {
                        self.backend.append_to_commandline(argument);
                    }
                    None => ()
                }
            }

            "OVERWRITE_STRING" => {
                let pattern = match arguments.get(0) {
                    Some(argument) => {
                        argument
                    }
                    None => {
                        ""
                    }
                };
                let repeat = self.grab_register(1);
                self.ci_overwrite_string(pattern, repeat)?;

            }

            //"OVERWRITE_RAW" => {
            //    let repeat = self.grab_register(1);
            //    let pattern = match arguments.get(0) {
            //        Some(arg) => {
            //            arg.as_bytes().to_vec()
            //        }
            //        None => {
            //            vec![]
            //        }
            //    };
            //    self.ci_overwrite_bytes(&pattern, repeat)?;
            //}

            "DECREMENT" => {
                let repeat = max(1, self.grab_register(1));
                self.ci_decrement(repeat);
            }

            "INCREMENT" => {
                let repeat = self.grab_register(1);
                self.ci_increment(repeat);
            }

            "RUN_CUSTOM_COMMAND" => {
                match self.backend.fetch_commandline_register() {
                    Some(new_command) => {
                        self.query(&new_command)?;
                    }
                    None => ()
                }
                self.set_context("DEFAULT");
            }

            "KILL" => {
                self.running = false;
            }

            "QUIT" => {
                //TODO in later version: Prevent quitting when there are unsaved changes
                self.running = false;
            }

            "SAVE" => {
                match arguments.get(0) {
                    Some(arg) => {
                        self.ci_save(Some(&arg));
                    }
                    None => {
                        self.ci_save(None);
                    }
                }
            }

            "SAVEQUIT" => {
                self.ci_save(None);
                self.running = false;
            }

            "TOGGLE_CONVERTER" => {
                let new_converter = match self.backend.get_active_converter_ref() {
                    ConverterRef::BIN => {
                        ConverterRef::HEX
                    }
                    ConverterRef::HEX => {
                        ConverterRef::DEC
                    }
                    ConverterRef::DEC => {
                        ConverterRef::BIN
                    }
                };

                self.backend.set_active_converter(new_converter);
                self.__resize_hook();
            }

            "SET_WIDTH" => {
                match arguments.get(0) {
                    Some(argument) => {
                        match string_to_integer(argument) {
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
                    Some(argument) => {
                        match string_to_integer(argument) {
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
                        let mut digit;
                        for character in argument.chars() {
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

            "RELOAD" => {
                let path = match self.backend.get_active_file_path() {
                    Some(_path) => {
                        Ok(_path.clone())
                    }
                    None => {
                        Err(SbyteError::PathNotSet)
                    }
                }?;

                self.backend.load_file(&path);
                self.resize_backend_viewport();
            }

            _ => {
                // Unknown
            }
        }

        Ok(())
    }

    pub fn load_config(&mut self, file_path: &str) -> Result<(), SbyteError> {
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
                match file.read(&mut buffer) {
                    Ok(_) => { }
                    Err(_e) => {
                        Err(SbyteError::ReadFail)?;
                    }
                }

                let working_cmds: Vec<&str> = std::str::from_utf8(buffer.as_slice()).unwrap().split("\n").collect();
                for cmd in working_cmds.iter() {
                    if cmd.len() > 0 {
                        self.query(cmd)?;
                    }
                }
            }
            Err(_e) => { }
        }

        Ok(())
    }

    fn query(&mut self, query: &str) -> Result<(), SbyteError> {
        let result = match self.backend.try_command(query) {
            Ok((funcref, args)) => {
                let mut str_args = Vec::new();
                for arg in args.iter() {
                    str_args.push(arg.as_str());
                }
                self.send_command(&funcref, str_args.as_slice())
            }
            Err(e) => {
                Err(e)
            }
        };

        match result {
            Ok(_) => {}
            Err(e) => {
                self.backend.set_user_error_msg(&format!("{:?}", e));
            }
        }

        Ok(())
    }

    fn resize_backend_viewport(&mut self) {
        let cursor_offset = self.backend.get_cursor_real_offset();
        let cursor_length = self.backend.get_cursor_real_length();

        self.backend.set_viewport_offset(0);
        self.backend.set_cursor_length(1);
        self.backend.set_cursor_offset(0);

        let screensize = self.frontend.size();
        let display_ratio = self.backend.get_display_ratio() as f64;
        let r: f64 = 1f64 / display_ratio;
        let a: f64 = 1f64 - (1f64 / (r + 1f64));
        let mut base_width = ((screensize.0 as f64 - 1f64) * a) as usize;
        match self.get_locked_viewport_width() {
            Some(locked_width) => {
                base_width = min(base_width, locked_width);
            }
            None => ()
        }

        let height = self.frontend.get_viewport_height();

        self.backend.set_viewport_size(base_width, height);
        self.backend.set_cursor_offset(cursor_offset);
        self.backend.set_cursor_length(cursor_length);
    }

    fn __resize_hook(&mut self) {
        self.resize_backend_viewport();

    }

    fn auto_resize(&mut self) {
        if self.frontend.auto_resize() {
            let delay = time::Duration::from_nanos(1_000);
            thread::sleep(delay);
            self.__resize_hook();
        }
    }

    fn ci_subcursor_left(&mut self) -> Result<(), SbyteError> {
        self.backend.subcursor_prev_digit();
        Ok(())
    }
    fn ci_subcursor_right(&mut self) -> Result<(), SbyteError> {
        self.backend.subcursor_next_digit();
        Ok(())
    }


    fn ci_replace_digit(&mut self, digit: char) -> Result<(), SbyteError> {
        self.backend.replace_digit(digit)?;
        self.backend.subcursor_next_digit();

        if self.backend.get_subcursor_offset() == 0
        && self.backend.get_cursor_length() == 1 {
            self.backend.cursor_next_byte();
        }

        Ok(())
    }

    fn ci_cursor_up(&mut self, repeat: usize) {
        let cursor_offset = self.backend.get_cursor_offset();
        self.backend.set_cursor_offset(cursor_offset);
        self.backend.set_cursor_length(1);
        for _ in 0 .. repeat {
            self.backend.cursor_prev_line();
        }

    }

    fn ci_cursor_down(&mut self, repeat: usize) {
        let end_of_cursor = self.backend.get_cursor_offset() + self.backend.get_cursor_length();
        self.backend.set_cursor_length(1);
        self.backend.set_cursor_offset(end_of_cursor - 1);
        for _ in 0 .. repeat {
            self.backend.cursor_next_line();
        }

    }

    fn ci_cursor_left(&mut self, repeat: usize) {
        let cursor_offset = self.backend.get_cursor_offset();
        self.backend.set_cursor_offset(cursor_offset);
        self.backend.set_cursor_length(1);
        for _ in 0 .. repeat {
            self.backend.cursor_prev_byte();
        }

    }

    fn ci_cursor_right(&mut self, repeat: usize) {
        // Jump positon to the end of the cursor before moving it right
        let end_of_cursor = self.backend.get_cursor_offset() + self.backend.get_cursor_length();
        self.backend.set_cursor_offset(end_of_cursor - 1);
        self.backend.set_cursor_length(1);

        for _ in 0 .. repeat {
            self.backend.cursor_next_byte();
        }

    }

    fn ci_cursor_length_up(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_decrease_length_by_line();
        }

    }

    fn ci_cursor_length_down(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_increase_length_by_line();
        }

    }

    fn ci_cursor_length_left(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_decrease_length();
        }

    }

    fn ci_cursor_length_right(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_increase_length();
        }

    }

    fn ci_yank(&mut self) {
        self.backend.copy_selection();
        self.backend.set_cursor_length(1);

    }

    fn ci_jump_to_position(&mut self, new_offset: usize) {

        let content_length = self.backend.len();
        if new_offset <= content_length {
            self.backend.set_cursor_length(1);
            self.backend.set_cursor_offset(new_offset);
        } else {
            self.backend.set_user_error_msg(&format!("Out of Bounds: {} < {}", new_offset, content_length));
        }
    }

    fn ci_bitwise_not(&mut self) -> Result<(), SbyteError> {
        self.backend.bitwise_not()?;

        Ok(())
    }

    fn ci_apply_nand_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.backend.apply_nand_mask(mask)?;

        Ok(())
    }

    fn ci_apply_nor_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.backend.apply_nor_mask(mask)?;

        Ok(())
    }

    fn ci_apply_xor_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.backend.apply_xor_mask(mask)?;

        Ok(())
    }

    fn ci_apply_or_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.backend.apply_or_mask(mask)?;

        Ok(())
    }

    fn ci_apply_and_mask(&mut self, mask: &[u8]) -> Result<(), SbyteError> {
        self.backend.apply_and_mask(mask)?;

        Ok(())
    }

    fn ci_replace(&mut self, old_pattern: &str, new_pattern: &str) {
        let result = self.backend.replace(old_pattern, new_pattern.as_bytes());

        match result {
            Ok(hits) => {
                if hits.len() > 0 {
                    self.backend.set_user_msg(&format!("Replaced {} matches", hits.len()));
                } else {
                    self.backend.set_user_error_msg(&format!("Pattern \"{}\" not found", old_pattern));
                }
            }
            Err(_e) => {
                // TODO
            }
        }
    }

    fn ci_jump_to_previous(&mut self, argument: Option<&str>, repeat: usize) {
        let current_offset = self.backend.get_cursor_offset();

        let option_pattern: Option<String> = match argument {
            Some(pattern) => { // argument was given, use that
                Some(pattern.to_string())
            }
            None => { // No argument was given, check history
                match self.backend.get_search_history().last() {
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
            Some(string_rep) => {
                self.backend.add_search_history(string_rep.clone());
                match self.backend.find_nth_before(&string_rep, current_offset, repeat) {
                    Ok(result) => {
                        match result {
                            Some(new_offset) => {
                                self.backend.set_cursor_length((new_offset.1 - new_offset.0) as isize);
                                self.backend.set_cursor_offset(new_offset.0);
                                self.backend.set_user_msg(&format!("Found \"{}\" at byte {}", string_rep, new_offset.0))
                            }
                            None => {
                                self.backend.set_user_error_msg(&format!("Pattern \"{}\" not found", string_rep));
                            }
                        }
                    }
                    Err(_e) => {
                        self.backend.set_user_error_msg(&format!("Pattern \"{}\" is Invalid", string_rep));
                    }
                }
            }
            None => {
                self.backend.set_user_error_msg("Need a pattern to search");
            }
        }


    }

    fn ci_jump_to_next(&mut self, argument: Option<&str>, repeat: usize) {
        let current_offset = self.backend.get_cursor_offset();

        let option_pattern: Option<String> = match argument {
            Some(pattern) => { // argument was given, use that
                Some(pattern.to_string())
            }
            None => { // No argument was given, check history
                match self.backend.get_search_history().last() {
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
            Some(string_rep) => {
                self.backend.add_search_history(string_rep.clone());
                match self.backend.find_nth_after(&string_rep, current_offset, repeat) {
                    Ok(result) => {
                        match result {
                            Some(new_offset) => {
                                self.backend.set_cursor_length((new_offset.1 - new_offset.0) as isize);
                                self.backend.set_cursor_offset(new_offset.0);
                                self.backend.set_user_msg(&format!("Found \"{}\" at byte {}", string_rep, new_offset.0))
                            }
                            None => {
                                self.backend.set_user_error_msg(&format!("Pattern \"{}\" not found", string_rep));
                            }
                        }
                    }
                    Err(_e) => {
                        self.backend.set_user_error_msg(&format!("Pattern \"{}\" is Invalid", string_rep));
                    }
                }
            }
            None => {
                self.backend.set_user_error_msg("Need a pattern to search");
            }
        }


    }

    fn ci_delete(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();

        let mut removed_bytes = Vec::new();
        for _ in 0 .. repeat {
            removed_bytes.extend(self.backend.remove_bytes_at_cursor().iter().copied());
        }
        self.backend.copy_to_clipboard(removed_bytes);
        self.backend.set_cursor_length(1);
    }

    fn ci_backspace(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        let adj_repeat = min(offset, repeat);

        self.ci_cursor_left(adj_repeat);

        // cast here is ok. repeat can't be < 0.
        self.backend.set_cursor_length(adj_repeat as isize);

        self.ci_delete(1);
    }

    fn ci_undo(&mut self, repeat: usize) {
        // viewport offset can be changed in undo, save it to check if a row update is needed
        let original_viewport_offset = self.backend.get_viewport_offset();

        let mut adj_repeat = 0;
        for i in 0 .. repeat {
            match self.backend.undo() {
                Err(SbyteError::EmptyStack) => {
                    break;
                }
                Err(_e) => {
                    // TODO
                    //Err(e)?;
                }
                Ok(_) => {
                    adj_repeat = i;
                }
            }
        }

        if adj_repeat > 1 {
            self.backend.set_user_msg(&format!("Undone x{}", adj_repeat));
        } else if repeat == 1 {
            self.backend.set_user_msg("Undid last change");
        } else {
            self.backend.set_user_msg("Nothing to undo");
        }


    }

    fn ci_redo(&mut self, repeat: usize) {
        // viewport offset can be changed in redo, save it to check if a row update is needed
        let original_viewport_offset = self.backend.get_viewport_offset();

        for _ in 0 .. repeat {
            match self.backend.redo() {
                Err(SbyteError::EmptyStack) => {
                    break;
                }
                Err(_) => {}
                Ok(_) => {}
            }
        }

    }

    fn ci_insert_string(&mut self, argument: &str, repeat: usize) -> Result<(), SbyteError> {
        match string_to_bytes(argument) {
            Ok(converted_bytes) => {
                self.ci_insert_bytes(&converted_bytes, repeat)
            }
            Err(e) => {
                self.backend.set_user_error_msg(&format!("Invalid Pattern: {}", argument.clone()));
                Err(e)
            }
        }
    }

    fn ci_insert_bytes(&mut self, bytes: &[u8], repeat: usize) -> Result<(), SbyteError> {
        let offset = self.backend.get_cursor_offset();
        for _ in 0 .. repeat {
            self.backend.insert_bytes_at_cursor(bytes)?;
        }

        self.ci_cursor_right(bytes.len() * repeat);

        Ok(())
    }

    fn ci_overwrite_string(&mut self, argument: &str, repeat: usize) -> Result<(), SbyteError> {
        match string_to_bytes(argument) {
            Ok(converted_bytes) => {
                self.ci_overwrite_bytes(&converted_bytes, repeat)
            }
            Err(e) => {
                self.backend.set_user_error_msg(&format!("Invalid Pattern: {}", argument.clone()));
                Err(e)
            }
        }
    }

    fn ci_overwrite_bytes(&mut self, bytes: &[u8], repeat: usize) -> Result<(), SbyteError> {
        let offset = self.backend.get_cursor_offset();
        for _ in 0 .. repeat {
            self.backend.overwrite_bytes_at_cursor(bytes)?;
            self.ci_cursor_right(bytes.len());
        }
        self.backend.set_cursor_length(1);

        Ok(())
    }

    fn ci_increment(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        let cursor_length = self.backend.get_cursor_length();
        for _ in 0 .. repeat {
            match self.backend.increment_byte(offset + (cursor_length - 1), cursor_length) {
                Err(SbyteError::OutOfBounds(_, _)) => {
                    break;
                }
                Err(_) => {}
                Ok(_) => {}
            }
        }


        let mut suboffset: usize = 0;
        let mut chunk;
        while offset > suboffset {
            chunk = self.backend.get_chunk(offset - suboffset, 1);
            if chunk.len() > 0 && (chunk[0] as u32) < (repeat >> (8 * suboffset)) as u32 {
                suboffset += 1;
            } else {
                break;
            }
        }

    }

    fn ci_decrement(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        let cursor_length = self.backend.get_cursor_length();
        for _ in 0 .. repeat {
            match self.backend.decrement_byte(offset + (cursor_length - 1), cursor_length) {
                Ok(_) => {}
                Err(SbyteError::OutOfBounds(_, _)) => {
                    break;
                }
                Err(_) => { }
            }
        }


        let mut chunk;
        let mut suboffset: usize = 0;
        while offset > suboffset {
            chunk = self.backend.get_chunk(offset - suboffset, 1);
            if chunk.len() > 0 && (chunk[0] as u32) > (repeat >> (8 * suboffset)) as u32 {
                suboffset += 1;
            } else {
                break;
            }
        }
    }

    fn ci_save(&mut self, path: Option<&str>) {
        match path {
            Some(string_path) => {
                match self.backend.save_as(&string_path) {
                    Ok(_) => {
                        self.backend.set_user_msg(&format!("Saved to file: {}", string_path));
                    }
                    Err(e) => {
                        self.backend.set_user_error_msg(&format!("{:?}", e));
                    }
                }
            }
            None => {
                let result = match self.backend.save() {
                    Ok(_) => {
                        match self.backend.get_active_file_path() {
                            Some(file_path) => {
                                Ok(format!("Saved to file: {}", file_path))
                            }
                            None => {
                                Err("File path not set".to_string())
                            } // Unreachable
                        }
                    }
                    Err(_) => {
                        Err("No path specified".to_string())
                    }
                };

                match result {
                    Ok(msg) => {
                        self.backend.set_user_msg(&msg);
                    }
                    Err(msg) => {
                        self.backend.set_user_error_msg(&msg);
                    }
                }
            }
        }
    }

    fn ci_lock_viewport_width(&mut self, new_width: usize) {
        self.lock_viewport_width(new_width);
        self.__resize_hook();
    }

    fn ci_unlock_viewport_width(&mut self) {
        self.unlock_viewport_width();
        self.__resize_hook();
    }

    fn get_locked_viewport_width(&mut self) -> Option<usize> {
        self.locked_viewport_width
    }

    fn unlock_viewport_width(&mut self) {
        self.locked_viewport_width = None;
    }

    fn lock_viewport_width(&mut self, new_width: usize) {
        self.locked_viewport_width = Some(new_width);
    }

    fn user_feedback_ready(&mut self) -> bool {
        self.backend.get_user_msg().is_some() || self.backend.get_user_error_msg().is_some()
    }

    fn set_context(&mut self, new_context: &str) {
        self.backend.set_subcursor_offset(0);
        match self.inputter.try_lock() {
            Ok(ref mut mutex) => {
                mutex.set_context(new_context);
            }
            Err(_e) => ()
        }
        self.frontend.set_input_context(new_context);
    }
}
