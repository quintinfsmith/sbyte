use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::sync::{Mutex, Arc};

pub mod tests;
pub mod inputter;

//TODO Move string_to_integer
use super::shell::{Shell, parse_words};
use super::editor::SbyteError;
use super::editor::formatter::*;
use super::console_displayer::FrontEnd;
use inputter::Inputter;

use std::{time, thread};
use std::collections::HashMap;


pub struct InputInterface {
    shell: Shell,
    frontend: FrontEnd,
    inputter: Arc<Mutex<Inputter>>,

    running: bool,
    key_map: HashMap<&'static str, Vec<u8>>
}

impl InputInterface {
    pub fn new(shell: Shell, frontend: FrontEnd) -> InputInterface {
        let mut interface = InputInterface {
            running: false,
            inputter: Arc::new(Mutex::new(InputInterface::new_inputter())),

            key_map: InputInterface::build_key_map(),
            shell,
            frontend
        };

        interface.setup_default_controls().ok().unwrap();
        interface.auto_resize();

        interface
    }

    fn setup_default_controls(&mut self) -> Result<(), SbyteError> {
        // Default Controls
        self.hook_assign_mode_input(&["DEFAULT", "TOGGLE_FORMATTER", "EQUALS"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_DOWN", "J_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_UP", "K_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_LEFT", "H_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_RIGHT", "L_LOWER"]);

        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_DOWN", "ARROW_DOWN"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_UP", "ARROW_UP"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_LEFT", "ARROW_LEFT"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_RIGHT", "ARROW_RIGHT"]);

        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_LENGTH_DOWN", "J_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_LENGTH_UP", "K_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_LENGTH_LEFT", "H_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "CURSOR_LENGTH_RIGHT", "L_UPPER"]);

        self.hook_assign_mode_input(&["DEFAULT", "JUMP_TO_REGISTER", "G_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "JUMP_TO_NEXT_HIGHLIGHTED", "GREATERTHAN"]);
        self.hook_assign_mode_input(&["DEFAULT", "JUMP_TO_PREVIOUS_HIGHLIGHTED", "LESSTHAN"]);
        self.hook_assign_mode_input(&["DEFAULT", "POINTER_BE_JUMP", "R_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "POINTER_LE_JUMP", "T_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "DELETE", "X_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "YANK", "Y_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "PASTE", "P_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "UNDO", "U_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "REDO", "CTRL+R"]);
        self.hook_assign_mode_input(&["DEFAULT", "CLEAR_REGISTER", "ESCAPE"]);
        self.hook_assign_mode_input(&["DEFAULT", "INCREMENT", "PLUS"]);
        self.hook_assign_mode_input(&["DEFAULT", "DECREMENT", "DASH"]);
        self.hook_assign_mode_input(&["DEFAULT", "BACKSPACE", "BACKSPACE"]);
        self.hook_assign_mode_input(&["DEFAULT", "DELETE", "DELETE"]);

        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_INSERT_ASCII", "I_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_OVERWRITE", "O_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_OVERWRITE_ASCII", "O_UPPER"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_APPEND", "A_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_SEARCH", "SLASH"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_CMD", "COLON"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_MASK_OR", "BAR"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_MASK_AND", "AMPERSAND"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_MASK_XOR", "CARET"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_RECORD_KEY", "Q_LOWER"]);
        self.hook_assign_mode_input(&["DEFAULT", "MODE_SET_PLAYBACK_KEY", "AT"]);
        self.hook_assign_mode_input(&["DEFAULT", "BITWISE_NOT", "TILDE"]);

        self.hook_assign_mode_input(&["OVERWRITE_BIN", "SUBCURSOR_LEFT", "H_LOWER"]);
        self.hook_assign_mode_input(&["OVERWRITE_BIN", "SUBCURSOR_RIGHT", "L_LOWER"]);
        self.hook_assign_mode_input(&["OVERWRITE_DEC", "SUBCURSOR_LEFT", "H_LOWER"]);
        self.hook_assign_mode_input(&["OVERWRITE_DEC", "SUBCURSOR_RIGHT", "L_LOWER"]);
        self.hook_assign_mode_input(&["OVERWRITE_HEX", "SUBCURSOR_LEFT", "H_LOWER"]);
        self.hook_assign_mode_input(&["OVERWRITE_HEX", "SUBCURSOR_RIGHT", "L_LOWER"]);

        //self.send_command("ASSIGN_INPUT", &["MODE_SET_INSERT", "I_LOWER"])?;
        //self.send_command("ASSIGN_INPUT", &["MODE_SET_INSERT_SPECIAL", "I_UPPER"])?;
        //self.send_command("ASSIGN_INPUT", &["MODE_SET_OVERWRITE_SPECIAL", "O_UPPER"])?;

        self.hook_assign_mode_input(&["INSERT_ASCII", "MODE_SET_DEFAULT", "ESCAPE"]);
        self.hook_assign_mode_input(&["OVERWRITE_ASCII", "MODE_SET_DEFAULT", "ESCAPE"]);
        self.hook_assign_mode_input(&["OVERWRITE_HEX", "MODE_SET_DEFAULT", "ESCAPE"]);
        self.hook_assign_mode_input(&["OVERWRITE_DEC", "MODE_SET_DEFAULT", "ESCAPE"]);
        self.hook_assign_mode_input(&["OVERWRITE_BIN", "MODE_SET_DEFAULT", "ESCAPE"]);

        let mut ascii_map: HashMap<Vec<u8>, String> = HashMap::new();
        for (key, value) in self.key_map.iter() {
            ascii_map.insert(value.to_vec(), key.to_string());
        }

        for c in b"01".iter() {
            let strrep = std::str::from_utf8(&[*c]).unwrap().to_string();
            let keycode = ascii_map.get(&vec![*c]).unwrap();
            self.hook_assign_mode_input(&["OVERWRITE_BIN", "OVERWRITE_DIGIT", &keycode, &strrep]);
        }

        for c in b"0123456789".iter() {
            let strrep = std::str::from_utf8(&[*c]).unwrap().to_string();
            let keycode = ascii_map.get(&vec![*c]).unwrap();
            self.hook_assign_mode_input(&["OVERWRITE_DEC", "OVERWRITE_DIGIT", &keycode, &strrep]);
            self.hook_assign_mode_input(&["INSERT_ASCII", "INSERT_STRING", &keycode, &strrep]);
            self.hook_assign_mode_input(&["DEFAULT", "APPEND_TO_REGISTER", &keycode, &strrep]);
        }

        for c in b"0123456789abcdef".iter() {
            let strrep = std::str::from_utf8(&[*c]).unwrap().to_string();
            let keycode = ascii_map.get(&vec![*c]).unwrap();
            self.hook_assign_mode_input(&["OVERWRITE_HEX", "OVERWRITE_DIGIT", &keycode, &strrep]);
        }

        for i in 32 .. 127 {
            let strrep = std::str::from_utf8(&[i]).unwrap().to_string();
            let keycode = ascii_map.get(&vec![i]).unwrap();

            self.hook_assign_mode_input(&["INSERT_ASCII", "INSERT_STRING", &keycode, &strrep]);
            self.hook_assign_mode_input(&["OVERWRITE_ASCII", "OVERWRITE_STRING", &keycode, &strrep]);
            self.hook_assign_mode_input(&["CMD", "APPEND_TO_COMMANDLINE", &keycode, &strrep]);
        }

        self.hook_assign_mode_input(&["CMD", "RUN_CUSTOM_COMMAND", "LINE_FEED"]);
        self.hook_assign_mode_input(&["CMD", "MODE_SET_DEFAULT", "ESCAPE"]);
        self.hook_assign_mode_input(&["CMD", "CMDLINE_BACKSPACE", "BACKSPACE"]);

        Ok(())
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
                            mutex.input(buffer[0]);
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
        Inputter::new()
    }

    pub fn main(&mut self) -> Result<(), SbyteError> {
        self.spawn_ctrl_c_daemon();
        self.auto_resize();
        let mut _input_daemon = self.spawn_input_daemon();

        let fps = 30.0;
        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);

        self.running = true;

        let mut result = Ok(());

        while self.running {
            self.frontend.tick(&mut self.shell).ok();

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
                    match self.send_command(&funcref, str_args.as_slice()) {
                        Ok(()) => {}
                        Err(SbyteError::KillSignal) => {
                            self.running = false;
                        }
                        Err(e) => {
                            result = Err(e);
                            self.running = false;
                            break;
                        }
                    }
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

        match self.frontend.kill() {
            Ok(()) => { }
            Err(e) => {
                result = Err(e);
            }
        }

        result
    }

    pub fn build_key_map() -> HashMap<&'static str, Vec<u8>> {
        let mut key_map = HashMap::new();
        // Common control characters
        key_map.insert("BACKSPACE", vec![b'\x7F']);
        key_map.insert("TAB", vec![b'\x09']);
        key_map.insert("LINE_FEED", vec![b'\x0A']);
        key_map.insert("RETURN", vec![b'\x0D']);
        key_map.insert("ESCAPE", vec![b'\x1B']);
        key_map.insert("ARROW_UP", vec![b'\x1B', b'[', b'A']);
        key_map.insert("ARROW_LEFT", vec![b'\x1B', b'[', b'D']);
        key_map.insert("ARROW_DOWN", vec![b'\x1B', b'[', b'B']);
        key_map.insert("ARROW_RIGHT", vec![b'\x1B', b'[', b'C']);
        key_map.insert("DELETE", vec![b'\x1B', b'[', b'3', b'\x7e']);

        // lesser control characters
        key_map.insert("NULL", vec![b'\x00']);
        key_map.insert("STX", vec![b'\x01']);
        key_map.insert("SOT", vec![b'\x02']);
        key_map.insert("ETX", vec![b'\x03']);
        key_map.insert("EOT", vec![b'\x04']);
        key_map.insert("ENQ", vec![b'\x05']);
        key_map.insert("ACK", vec![b'\x06']);
        key_map.insert("BELL", vec![b'\x07']);
        key_map.insert("VTAB", vec![b'\x0B']);
        key_map.insert("FORM_FEED", vec![b'\x0C']);
        key_map.insert("SHIFT_OUT", vec![b'\x0E']);
        key_map.insert("SHIFT_IN", vec![b'\x0F']);
        key_map.insert("DATA_LINK_ESCAPE", vec![b'\x10']);
        key_map.insert("XON", vec![b'\x11']);
        key_map.insert("CTRL+R", vec![b'\x12']);
        key_map.insert("XOFF", vec![b'\x13']);
        key_map.insert("DC4", vec![b'\x14']);
        key_map.insert("NAK", vec![b'\x15']);
        key_map.insert("SYN", vec![b'\x16']);
        key_map.insert("ETB", vec![b'\x17']);
        key_map.insert("CANCEL", vec![b'\x18']);
        key_map.insert("EM", vec![b'\x19']);
        key_map.insert("SUB", vec![b'\x1A']);
        key_map.insert("FILE_SEPARATOR", vec![b'\x1C']);
        key_map.insert("GROUP_SEPARATOR", vec![b'\x1D']);
        key_map.insert("RECORD_SEPARATOR", vec![b'\x1E']);
        key_map.insert("UNITS_EPARATOR", vec![b'\x1F']);

        // Regular character Keys
        key_map.insert("ONE", vec![b'1']);
        key_map.insert("TWO", vec![b'2']);
        key_map.insert("THREE", vec![b'3']);
        key_map.insert("FOUR", vec![b'4']);
        key_map.insert("FIVE", vec![b'5']);
        key_map.insert("SIX", vec![b'6']);
        key_map.insert("SEVEN", vec![b'7']);
        key_map.insert("EIGHT", vec![b'8']);
        key_map.insert("NINE", vec![b'9']);
        key_map.insert("ZERO", vec![b'0']);
        key_map.insert("BANG", vec![b'!']);
        key_map.insert("AT", vec![b'@']);
        key_map.insert("OCTOTHORPE", vec![b'#']);
        key_map.insert("DOLLAR", vec![b'$']);
        key_map.insert("PERCENT", vec![b'%']);
        key_map.insert("CARET", vec![b'^']);
        key_map.insert("AMPERSAND", vec![b'&']);
        key_map.insert("ASTERISK", vec![b'*']);
        key_map.insert("PARENTHESIS_OPEN", vec![b'(']);
        key_map.insert("PARENTHESIS_CLOSE", vec![b')']);
        key_map.insert("BRACKET_OPEN", vec![b'[']);
        key_map.insert("BRACKET_CLOSE", vec![b']']);
        key_map.insert("BRACE_OPEN", vec![b'{']);
        key_map.insert("BRACE_CLOSE", vec![b'}']);
        key_map.insert("BAR", vec![b'|']);
        key_map.insert("BACKSLASH", vec![b'\\']);
        key_map.insert("COLON", vec![b':']);
        key_map.insert("SEMICOLON", vec![b';']);
        key_map.insert("QUOTE", vec![b'\"']);
        key_map.insert("APOSTROPHE", vec![b'\'']);
        key_map.insert("LESSTHAN", vec![b'<']);
        key_map.insert("GREATERTHAN", vec![b'>']);
        key_map.insert("COMMA", vec![b',']);
        key_map.insert("PERIOD", vec![b'.']);
        key_map.insert("SLASH", vec![b'/']);
        key_map.insert("QUESTIONMARK", vec![b'?']);
        key_map.insert("DASH", vec![b'-']);
        key_map.insert("UNDERSCORE", vec![b'_']);
        key_map.insert("SPACE", vec![b' ']);
        key_map.insert("PLUS", vec![b'+']);
        key_map.insert("EQUALS", vec![b'=']);
        key_map.insert("TILDE", vec![b'~']);
        key_map.insert("BACKTICK", vec![b'`']);
        key_map.insert("A_UPPER", vec![b'A']);
        key_map.insert("B_UPPER", vec![b'B']);
        key_map.insert("C_UPPER", vec![b'C']);
        key_map.insert("D_UPPER", vec![b'D']);
        key_map.insert("E_UPPER", vec![b'E']);
        key_map.insert("F_UPPER", vec![b'F']);
        key_map.insert("G_UPPER", vec![b'G']);
        key_map.insert("H_UPPER", vec![b'H']);
        key_map.insert("I_UPPER", vec![b'I']);
        key_map.insert("J_UPPER", vec![b'J']);
        key_map.insert("K_UPPER", vec![b'K']);
        key_map.insert("L_UPPER", vec![b'L']);
        key_map.insert("M_UPPER", vec![b'M']);
        key_map.insert("N_UPPER", vec![b'N']);
        key_map.insert("O_UPPER", vec![b'O']);
        key_map.insert("P_UPPER", vec![b'P']);
        key_map.insert("Q_UPPER", vec![b'Q']);
        key_map.insert("R_UPPER", vec![b'R']);
        key_map.insert("S_UPPER", vec![b'S']);
        key_map.insert("T_UPPER", vec![b'T']);
        key_map.insert("U_UPPER", vec![b'U']);
        key_map.insert("V_UPPER", vec![b'V']);
        key_map.insert("W_UPPER", vec![b'W']);
        key_map.insert("X_UPPER", vec![b'X']);
        key_map.insert("Y_UPPER", vec![b'Y']);
        key_map.insert("Z_UPPER", vec![b'Z']);
        key_map.insert("A_LOWER", vec![b'a']);
        key_map.insert("B_LOWER", vec![b'b']);
        key_map.insert("C_LOWER", vec![b'c']);
        key_map.insert("D_LOWER", vec![b'd']);
        key_map.insert("E_LOWER", vec![b'e']);
        key_map.insert("F_LOWER", vec![b'f']);
        key_map.insert("G_LOWER", vec![b'g']);
        key_map.insert("H_LOWER", vec![b'h']);
        key_map.insert("I_LOWER", vec![b'i']);
        key_map.insert("J_LOWER", vec![b'j']);
        key_map.insert("K_LOWER", vec![b'k']);
        key_map.insert("L_LOWER", vec![b'l']);
        key_map.insert("M_LOWER", vec![b'm']);
        key_map.insert("N_LOWER", vec![b'n']);
        key_map.insert("O_LOWER", vec![b'o']);
        key_map.insert("P_LOWER", vec![b'p']);
        key_map.insert("Q_LOWER", vec![b'q']);
        key_map.insert("R_LOWER", vec![b'r']);
        key_map.insert("S_LOWER", vec![b's']);
        key_map.insert("T_LOWER", vec![b't']);
        key_map.insert("U_LOWER", vec![b'u']);
        key_map.insert("V_LOWER", vec![b'v']);
        key_map.insert("W_LOWER", vec![b'w']);
        key_map.insert("X_LOWER", vec![b'x']);
        key_map.insert("Y_LOWER", vec![b'y']);
        key_map.insert("Z_LOWER", vec![b'z']);

        key_map
    }

    fn send_command(&mut self, funcref: &str, arguments: &[&str]) -> Result<(), SbyteError> {
        let mut output = Ok(());
        match funcref {
            "ASSIGN_INPUT" => {
                let mut alt_args = vec!["DEFAULT"];
                for arg in arguments.iter() {
                    alt_args.push(*arg);
                }
                self.hook_assign_mode_input(&alt_args);
            }

            "ASSIGN_MODE_INPUT" => {
                self.hook_assign_mode_input(arguments);
            }

            "MODE_SET_INSERT_ASCII" => {
                self.set_context("INSERT_ASCII");
            }

            "MODE_SET_INSERT_SPECIAL" => {
                self.set_context("CMD");
                self.shell.buffer_push("insert ");
            }

            "MODE_SET_OVERWRITE_SPECIAL" => {
                self.set_context("CMD");
                self.shell.buffer_push("overwrite ");
            }

            "MODE_SET_OVERWRITE" => {
                match self.shell.get_editor().get_active_formatter_ref() {
                    FormatterRef::BIN => {
                        self.set_context("OVERWRITE_BIN");
                    }
                    FormatterRef::HEX => {
                        self.set_context("OVERWRITE_HEX");
                    }
                    FormatterRef::DEC => {
                        self.set_context("OVERWRITE_DEC");
                    }
                };
                //self.editor.set_user_msg("--OVERWRITE--");
            }

            "MODE_SET_OVERWRITE_ASCII" => {
                self.set_context("OVERWRITE_ASCII");
                //self.editor.set_user_msg("--OVERWRITE--");
            }

            "MODE_SET_APPEND" => {
                self.send_command("MODE_SET_INSERT_ASCII", arguments)?;
                self.send_command("CURSOR_RIGHT", &[])?;
            }

            "MODE_SET_DEFAULT" => {
                self.set_context("DEFAULT");
            }

            "MODE_SET_CMD" => {
                self.set_context("CMD");
            }

            "MODE_SET_MASK_AND" => {
                self.set_context("CMD");
                self.shell.buffer_push("and");
            }
            "MODE_SET_MASK_OR" => {
                self.set_context("CMD");
                self.shell.buffer_push("or ");
            }
            "MODE_SET_MASK_XOR" => {
                self.set_context("CMD");
                self.shell.buffer_push("xor ");
            }

            "MODE_SET_SEARCH" => {
                self.set_context("CMD");
                self.shell.buffer_push("find ");
            }

            "MODE_SET_RECORD_KEY" => {
                if self.shell.is_recording() {
                    self.shell.try_command("rec", &[])?;
                } else {
                    self.set_context("CMD");
                    self.shell.buffer_push("rec ");
                }
            }

            "MODE_SET_PLAYBACK_KEY" => {
                self.set_context("CMD");
                self.shell.buffer_push("play ");
            }
/////////////////////////////////////////////////////////////

            something_else => {
                output = match self.shell.try_command(something_else, arguments) {
                    Ok(()) => {
                        if something_else == "RUN_CUSTOM_COMMAND" {
                            self.set_context("DEFAULT");
                        }
                        Ok(())
                    }
                    Err(SbyteError::InvalidCommand(failed_cmd)) => {
                        self.shell.log_error(&format!("bad command: '{}'", failed_cmd));
                        self.set_context("DEFAULT");
                        Ok(())
                    }
                    Err(SbyteError::BufferEmpty) => {
                        self.set_context("DEFAULT");
                        Ok(())
                    }
                    Err(e) => {
                        Err(e)
                    }
                };

            }
        }

        output
    }

    fn hook_assign_mode_input(&mut self, arguments: &[&str]) {
        if arguments.len() >= 3 {
            let mode_key: &str = arguments.get(0).unwrap();
            let new_funcref: &str = arguments.get(1).unwrap();

            let mut new_input_sequence: Vec<u8> = Vec::new();
            let mut sequence_is_valid = true;
            for word in arguments.get(2).unwrap().split(",") {
                match self.key_map.get(word) {
                    Some(seq) => {
                        for byte in seq.iter() {
                            new_input_sequence.push(*byte);
                        }
                    }
                    None => {
                        sequence_is_valid = false;
                        break;
                    }
                }
            }

            if sequence_is_valid {
                loop {
                    match self.inputter.try_lock() {
                        Ok(ref mut mutex) => {
                            mutex.assign_mode_command(mode_key, &new_input_sequence, &new_funcref, &arguments[3..]);
                            break;
                        }
                        Err(_e) => ()
                    }
                }
            } else {
                self.shell.log_error(&format!("Invalid input sequence: {}", arguments.get(2).unwrap()));
            }
        } else {
            self.shell.log_error("Mode key, function name & input sequence are required");
        }
    }


    //        "RELOAD" => {
    //            let path = match //self.editor.get_active_file_path() {
    //                Some(_path) => {
    //                    Ok(_path.clone())
    //                }
    //                None => {
    //                    Err(SbyteError::PathNotSet)
    //                }
    //            }?;

    //            //self.editor.load_file(&path);
    //            self.resize_editor_viewport();
    //        }

    //        _ => {
    //            // Unknown
    //        }


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

                for query in working_cmds.iter() {
                    let mut words = parse_words(query);

                    if !words.is_empty() {
                        let cmd = words.remove(0);
                        let mut args = vec![];
                        for word in words.iter() {
                            args.push(word.as_str());
                        }
                        self.send_command(&cmd, args.as_slice())?;
                    }

                }
            }
            Err(_e) => ()
        }

        Ok(())
    }


    fn auto_resize(&mut self) {
        self.frontend.auto_resize(&mut self.shell);
    }

    fn set_context(&mut self, new_context: &str) {
        self.shell.buffer_clear();

        if new_context == "CMD" {
            self.shell.buffer_push("");
        }

        if new_context != "DEFAULT" {
            self.shell.register_clear();
            self.shell.log_feedback(&format!("--{}--", new_context));
        }


        match self.inputter.try_lock() {
            Ok(ref mut mutex) => {
                mutex.set_context(new_context);
            }
            Err(_e) => ()
        }
        self.frontend.set_input_context(new_context);
    }
}
