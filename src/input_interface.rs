use std::cmp::{min, max};
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::sync::{Mutex, Arc};

//TODO Move string_to_integer
use super::sbyte_editor::{BackEnd, SbyteError, string_to_integer, string_to_bytes};
use super::sbyte_editor::editor::EditorError;
use super::sbyte_editor::editor::converter::*;
use super::sbyte_editor::flag::Flag;
use super::console_displayer::FrontEnd;

use std::{time, thread};
use std::collections::HashMap;


pub struct Inputter {
    input_managers: HashMap<String, InputNode>,
    input_buffer: Vec<u8>,
    context: String,
    context_switch: HashMap<String, String>
}

impl Inputter {
    pub fn new() -> Inputter {
        Inputter {
            input_managers: HashMap::new(),
            input_buffer: Vec::new(),
            context: "DEFAULT".to_string(),
            context_switch: HashMap::new()
        }
    }

    pub fn read_input(&mut self, input_byte: u8) -> Option<(String, String)> {
        let mut output = None;

        self.input_buffer.push(input_byte);

        let input_buffer = self.input_buffer.clone();
        let mut clear_buffer = false;
        match self.input_managers.get_mut(&self.context) {
            Some(root_node) => {
                let (cmd, completed_path) = root_node.fetch_command(input_buffer);
                match cmd {
                    Some(funcref) => {
                        match self.context_switch.get(&funcref) {
                            Some(new_context) => {
                                self.context = new_context.to_string();
                            }
                            None => ()
                        }

                        match std::str::from_utf8(&self.input_buffer) {
                            Ok(string) => {
                                output = Some((funcref, string.to_string()));
                            }
                            Err(_e) => {
                                output = Some((funcref, "".to_string()));
                            }
                        }
                    }
                    None => ()
                }
                clear_buffer = completed_path;
            }
            None => ()
        }

        if clear_buffer {
            self.input_buffer.drain(..);
        }

        output
    }

    pub fn assign_mode_command(&mut self, mode: &str, command_string: String, hook: &str) {
        let command_vec = command_string.as_bytes().to_vec();
        let mode_node = self.input_managers.entry(mode.to_string()).or_insert(InputNode::new());
        mode_node.assign_command(command_vec, hook);
    }

    pub fn set_context(&mut self, new_context: &str) {
        self.context = new_context.to_string();
    }

    pub fn assign_context_switch(&mut self, funcref: &str, context: &str) {
        self.context_switch.entry(funcref.to_string())
            .and_modify(|e| *e = context.to_string())
            .or_insert(context.to_string());
    }
}

struct InputNode {
    next_nodes: HashMap<u8, InputNode>,
    hook: Option<String>
}

impl InputNode {
    fn new() -> InputNode {
        InputNode {
            next_nodes: HashMap::new(),
            hook: None
        }
    }

    fn assign_command(&mut self, new_pattern: Vec<u8>, hook: &str) {
        let mut tmp_pattern = Vec::new();
        for byte in new_pattern.iter() {
            tmp_pattern.push(*byte);
        }

        if tmp_pattern.len() > 0 {
            let next_byte = tmp_pattern.remove(0);

            let next_node = self.next_nodes.entry(next_byte).or_insert(InputNode::new());
            next_node.assign_command(tmp_pattern, hook);

        } else {
            self.hook = Some(hook.to_string());
        }
    }

    fn fetch_command(&mut self, input_pattern: Vec<u8>) -> (Option<String>, bool) {
        match &self.hook {
            Some(hook) => {
                // Found, Clear buffer
                (Some(hook.to_string()), true)
            }
            None => {
                let mut tmp_pattern = input_pattern.clone();
                if tmp_pattern.len() > 0 {
                    let next_byte = tmp_pattern.remove(0);
                    match self.next_nodes.get_mut(&next_byte) {
                        Some(node) => {
                            node.fetch_command(tmp_pattern)
                        }
                        None => {
                            // Dead End, Clear Buffer
                            (None, true)
                        }
                    }
                } else {
                    // Nothing Found Yet, keep buffer
                    (None, false)
                }
            }
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

pub struct InputPipe {
    input_buffer: Vec<u8>,
    killed: bool
}
impl InputPipe {
    fn new() -> InputPipe {
        InputPipe {
            input_buffer: Vec::new(),
            killed: false
        }
    }

    fn push(&mut self, item: u8) {
        self.input_buffer.push(item);
    }

    fn kill(&mut self) {
        self.killed = true;
    }

    fn is_alive(&self) -> bool {
        !self.killed
    }

    fn get_buffer(&mut self) -> &mut Vec<u8> {
        &mut self.input_buffer
    }
}



pub struct InputInterface {
    backend: BackEnd,
    frontend: FrontEnd,
    inputter: Inputter,

    locked_viewport_width: Option<usize>,

    flag_input_context: Option<String>,
    input_pipe: Arc<Mutex<InputPipe>>,

    register: Option<usize>,

    running: bool
}


impl InputInterface {
    pub fn new(backend: BackEnd, frontend: FrontEnd) -> InputInterface {
        let mut interface = InputInterface {
            flag_input_context: None,
            input_pipe: Arc::new(Mutex::new(InputPipe::new())),
            locked_viewport_width: None,
            running: false,
            inputter: InputInterface::new_inputter(),
            register: None,

            backend,
            frontend
        };

        interface.setup_default_controls().ok().unwrap();
        interface.resize_backend_viewport();

        interface
    }

    fn setup_default_controls(&mut self) -> Result<(), Box<dyn Error>> {
        // Default Controls
        self.send_command("ASSIGN_INPUT", vec!["TOGGLE_CONVERTER".to_string(), "EQUALS".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_DOWN".to_string(), "J_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_UP".to_string(), "K_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_LEFT".to_string(), "H_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_RIGHT".to_string(), "L_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_LENGTH_DOWN".to_string(), "J_UPPER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_LENGTH_UP".to_string(), "K_UPPER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_LENGTH_LEFT".to_string(), "H_UPPER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CURSOR_LENGTH_RIGHT".to_string(), "L_UPPER".to_string()])?;

        self.send_command("ASSIGN_INPUT", vec!["JUMP_TO_REGISTER".to_string(), "G_UPPER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["DELETE".to_string(), "X_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["YANK".to_string(), "Y_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["PASTE".to_string(), "P_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["UNDO".to_string(), "U_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["REDO".to_string(), "CTRL+R".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["CLEAR_REGISTER".to_string(), "ESCAPE,ESCAPE".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["INCREMENT".to_string(), "PLUS".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["DECREMENT".to_string(), "DASH".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["BACKSPACE".to_string(), "BACKSPACE".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["DELETE".to_string(), "DELETE".to_string()])?;

        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_INSERT".to_string(), "I_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_INSERT_SPECIAL".to_string(), "I_UPPER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_OVERWRITE".to_string(), "O_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_OVERWRITE_SPECIAL".to_string(), "O_UPPER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_APPEND".to_string(), "A_LOWER".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_SEARCH".to_string(), "SLASH".to_string()])?;
        self.send_command("ASSIGN_INPUT", vec!["MODE_SET_CMD".to_string(), "COLON".to_string()])?;
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

    pub fn spawn_input_daemon(&mut self) -> std::thread::JoinHandle<()> {
        let input_pipe = self.input_pipe.clone();
        thread::spawn(move || {
            /////////////////////////////////
            // Rectmanager puts stdout in non-canonical mode,
            // so stdin will be char-by-char
            let stdout = io::stdout();
            let mut reader = io::stdin();
            let mut buffer: [u8; 1];
            stdout.lock().flush().unwrap();
            ////////////////////////////////

            let mut killed: bool = false;
            while ! killed {
                buffer = [0;1];
                reader.read_exact(&mut buffer).unwrap();
                match input_pipe.try_lock() {
                    Ok(ref mut mutex) => {
                        killed = !mutex.is_alive();
                        if ! killed {
                            for byte in buffer.iter() {
                                &mutex.push(*byte);
                            }
                        }
                    }
                    Err(_e) => ()
                }
            }
        })
    }

    pub fn spawn_ctrl_c_daemon(&mut self) {
        let signal_mutex = self.input_pipe.clone();
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


        // Enable ctrl-c
        inputter.assign_mode_command(mode_default, std::str::from_utf8(&[3]).unwrap().to_string(), "KILL");
        inputter.assign_mode_command(mode_insert, std::str::from_utf8(&[3]).unwrap().to_string(), "KILL");
        inputter.assign_mode_command(mode_overwrite, std::str::from_utf8(&[3]).unwrap().to_string(), "KILL");
        inputter.assign_mode_command(mode_cmd, std::str::from_utf8(&[3]).unwrap().to_string(), "KILL");


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


        inputter
    }

    pub fn main(&mut self) -> Result<(), Box<dyn Error>> {
        self.spawn_ctrl_c_daemon();
        self.auto_resize();
        let mut _input_daemon = self.spawn_input_daemon();

        let fps = 59.97;
        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);

        let mut command_pair: Option<(String, Vec<String>)>;
        self.running = true;
        while self.running {
            match self.frontend.tick(&self.backend) {
                Ok(_) => {
                    self.backend.unset_user_error_msg();
                    self.backend.unset_user_msg();
                }
                Err(boxed_error) => {
                    // To help debug ...
                    self.backend.set_user_error_msg(&format!("{:?}", boxed_error));
                }
            }

            command_pair = None;
            match self.input_pipe.try_lock() {
                Ok(ref mut mutex) => {

                    // Kill the main loop is the input loop dies
                    if ! mutex.is_alive() {
                        self.running = false;
                    }

                    let mut buffer = mutex.get_buffer();
                    for byte in buffer.drain(..) {
                        match self.inputter.read_input(byte) {
                            Some((funcref, input_sequence)) => {
                                command_pair = Some((funcref, vec![input_sequence]));
                            }
                            None => ()
                        }
                    }
                }
                Err(_e) => ()
            }

            match command_pair {
                Some((funcref, input_sequence)) => {
                    self.send_command(&funcref, input_sequence)?;
                }
                None => {
                    thread::sleep(delay);
                }
            }

            self.auto_resize();
        }

        // Kill input thread
        match self.input_pipe.try_lock() {
            Ok(ref mut mutex) => {
                mutex.kill();
            }
            Err(_e) => {}
        }

        self.frontend.kill()?;

        Ok(())
    }

    fn send_command(&mut self, funcref: &str, arguments: Vec<String>) -> Result<(), Box<dyn Error>> {
        match funcref {
            "ASSIGN_INPUT" => {
                let new_funcref: String = match arguments.get(0) {
                    Some(_new_funcref) => {
                        _new_funcref.clone()
                    }
                    None => {
                        "".to_string()
                    }
                };

                let new_input_string: String = match arguments.get(1) {
                    Some(_new_inputs) => {
                        let key_map = BackEnd::build_key_map();
                        let mut output = "".to_string();
                        for word in _new_inputs.split(",") {
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
                        "".to_string()
                    }
                };


                self.inputter.assign_mode_command("DEFAULT", new_input_string, &new_funcref);
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

            "CMDLINE_BACKSPACE" => {
                match self.backend.get_commandline_mut() {
                    Some(mut commandline) => {
                        if commandline.is_empty() {
                            self.frontend.raise_flag(Flag::UpdateOffset);
                        } else {
                            commandline.backspace();
                            self.frontend.raise_flag(Flag::DisplayCMDLine);
                        }
                    }
                    None => ()
                }
            }

            "YANK" => {
                self.ci_yank();
            }

            "PASTE" => {
                let repeat = self.grab_register(1);
                let to_paste = self.backend.get_clipboard();
                self.ci_insert_bytes(to_paste, repeat);
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

            "MODE_SET_INSERT" => {
                self.clear_register();
                self.backend.set_user_msg("--INSERT--");
            }

            "MODE_SET_OVERWRITE" => {
                self.clear_register();
                self.backend.set_user_msg("--OVERWRITE--");
            }

            "MODE_SET_APPEND" => {
                self.clear_register();
                self.ci_cursor_right(1);
                self.backend.set_user_msg("--INSERT--");
            }

            "MODE_SET_DEFAULT" => {
                self.clear_register();
                self.frontend.raise_flag(Flag::UpdateOffset);
                self.frontend.raise_flag(Flag::CursorMoved);
            }

            "MODE_SET_CMD" => {
                match self.backend.get_commandline_mut() {
                    Some(mut commandline) => {
                        commandline.clear_register();
                        self.frontend.raise_flag(Flag::DisplayCMDLine);
                    }
                    None => ()
                }
            }

            "MODE_SET_SEARCH" => {
                match self.backend.get_commandline_mut() {
                    Some(mut commandline) => {
                        commandline.set_register("find ");
                        self.frontend.raise_flag(Flag::DisplayCMDLine);
                    }
                    None => ()
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
                self.ci_insert_string(pattern, repeat);
            }

            "INSERT_RAW" => {
                let repeat = self.grab_register(1);
                let pattern = match arguments.get(0) {
                    Some(arg) => {
                        arg.as_bytes().to_vec()
                    }
                    None => {
                        vec![]
                    }
                };
                self.ci_insert_bytes(pattern, repeat);
            }

            "INSERT_TO_CMDLINE" => {
                match arguments.get(0) {
                    Some(argument) => {
                        match self.backend.get_commandline_mut() {
                            Some(mut commandline) => {
                                commandline.insert_to_register(argument);
                                commandline.move_cursor_right();
                                self.frontend.raise_flag(Flag::DisplayCMDLine);
                            }
                            None => ()
                        }
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
                self.ci_overwrite_string(pattern, repeat);

            }

            "OVERWRITE_RAW" => {
                let repeat = self.grab_register(1);
                let pattern = match arguments.get(0) {
                    Some(arg) => {
                        arg.as_bytes().to_vec()
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
                match self.backend.get_commandline_mut() {
                    Some(mut commandline) => {
                        match commandline.apply_register() {
                            Some(new_command) => {
                                self.query(&new_command)?;
                            }
                            None => {
                            }
                        };
                    }
                    None => ()
                }
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
                    _ => {
                        ConverterRef::HEX
                    }
                };

                self.backend.set_active_converter(new_converter);
                self.backend.adjust_viewport_offset();

                self.frontend.raise_flag(Flag::SetupDisplays);
                self.frontend.raise_flag(Flag::RemapActiveRows);
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

            _ => {
                // Unknown
            }
        }

        Ok(())
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
                for cmd in working_cmds.iter() {
                    self.query(cmd)?;
                }
            }
            Err(_e) => { }
        }

        Ok(())
    }

    fn query(&mut self, query: &str) -> Result<(), Box<dyn Error>> {
        let (funcref, args) = self.backend.try_command(query)?;
        self.send_command(&funcref, args)?;
        Ok(())
    }

    fn resize_backend_viewport(&mut self) {
        self.backend.set_viewport_offset(0);
        self.backend.set_cursor_offset(0);

        let mut screensize = self.frontend.size();
        let display_ratio = self.backend.get_display_ratio() as f64;
        let r: f64 = 1f64 / display_ratio;
        let a: f64 = 1f64 - (1f64 / (r + 1f64));
        let mut base_width = ((screensize.0 as f64) * a) as usize;
        match self.get_locked_viewport_width() {
            Some(locked_width) => {
                base_width = min(base_width, locked_width);
            }
            None => ()
        }

        let height = self.frontend.get_viewport_height();

        self.backend.set_viewport_size(base_width, height);
    }

    fn auto_resize(&mut self) {
        if self.frontend.auto_resize() {
            self.resize_backend_viewport();

            self.frontend.raise_flag(Flag::SetupDisplays);
            self.frontend.raise_flag(Flag::RemapActiveRows);
            self.frontend.raise_flag(Flag::ForceRerow);
        }
    }

    fn ci_cursor_up(&mut self, repeat: usize) {
        let cursor_offset = self.backend.get_cursor_offset();
        self.backend.set_cursor_offset(cursor_offset);
        self.backend.set_cursor_length(1);
        for _ in 0 .. repeat {
            self.backend.cursor_prev_line();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::UpdateOffset);
        self.frontend.raise_flag(Flag::CursorMoved);
    }

    fn ci_cursor_down(&mut self, repeat: usize) {
        let end_of_cursor = self.backend.get_cursor_offset() + self.backend.get_cursor_length();
        self.backend.set_cursor_length(1);
        self.backend.set_cursor_offset(end_of_cursor - 1);
        for _ in 0 .. repeat {
            self.backend.cursor_next_line();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::UpdateOffset);
        self.frontend.raise_flag(Flag::CursorMoved);
    }

    fn ci_cursor_left(&mut self, repeat: usize) {
        let cursor_offset = self.backend.get_cursor_offset();
        self.backend.set_cursor_offset(cursor_offset);
        self.backend.set_cursor_length(1);
        for _ in 0 .. repeat {
            self.backend.cursor_prev_byte();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::UpdateOffset);
        self.frontend.raise_flag(Flag::CursorMoved);

    }

    fn ci_cursor_right(&mut self, repeat: usize) {
        // Jump positon to the end of the cursor before moving it right
        let end_of_cursor = self.backend.get_cursor_offset() + self.backend.get_cursor_length();
        self.backend.set_cursor_offset(end_of_cursor - 1);
        self.backend.set_cursor_length(1);

        for _ in 0 .. repeat {
            self.backend.cursor_next_byte();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_cursor_length_up(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_decrease_length_by_line();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_cursor_length_down(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_increase_length_by_line();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_cursor_length_left(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_decrease_length();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_cursor_length_right(&mut self, repeat: usize) {
        for _ in 0 .. repeat {
            self.backend.cursor_increase_length();
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_yank(&mut self) {
        self.backend.copy_selection();
        self.backend.set_cursor_length(1);

        self.frontend.raise_flag(Flag::CursorMoved);
    }

    fn ci_jump_to_position(&mut self, new_offset: usize) {
        self.backend.set_cursor_length(1);
        self.backend.set_cursor_offset(new_offset);

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_jump_to_next(&mut self, argument: Option<&str>, repeat: usize) {
        let current_offset = self.backend.get_cursor_offset();
        let mut next_offset = current_offset;
        let mut new_cursor_length = self.backend.get_cursor_length();

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
                                self.backend.set_user_msg(&format!("Found \"{}\" at byte {}", string_rep, next_offset))
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


        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }


    fn ci_delete(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();

        let mut removed_bytes = Vec::new();
        for _ in 0 .. repeat {
            removed_bytes.extend(self.backend.remove_bytes_at_cursor().iter().copied());
        }
        self.backend.copy_to_clipboard(removed_bytes);
        self.backend.set_cursor_length(1);


        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
        self.flag_row_update_by_offset(offset);
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
                Ok(_) => {
                    adj_repeat = i;
                }
                Err(SbyteError::EmptyStack) => {
                    break;
                }
                Err(_e) => {
                    // TODO
                    //Err(e)?;
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


        let viewport_offset = self.backend.get_viewport_offset();
        if viewport_offset == original_viewport_offset {
            let (width, height) = self.backend.get_viewport_size();
            let start = viewport_offset / width;
            let end = height + start;

            for y in start .. end {
                self.frontend.raise_flag(Flag::UpdateRow(y));
            }
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_redo(&mut self, repeat: usize) {
        // viewport offset can be changed in redo, save it to check if a row update is needed
        let original_viewport_offset = self.backend.get_viewport_offset();

        for _ in 0 .. repeat {
            self.backend.redo();
        }

        let viewport_offset = self.backend.get_viewport_offset();
        if viewport_offset == original_viewport_offset {
            let (width, height) = self.backend.get_viewport_size();
            let start = viewport_offset / width;
            let end = height + start;

            for y in start .. end {
                self.frontend.raise_flag(Flag::UpdateRow(y));
            }
        }

        self.frontend.raise_flag(Flag::RemapActiveRows);
        self.frontend.raise_flag(Flag::CursorMoved);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_insert_string(&mut self, argument: &str, repeat: usize) {
        match string_to_bytes(argument.to_string()) {
            Ok(converted_bytes) => {
                self.ci_insert_bytes(converted_bytes.clone(), repeat);
            }
            Err(_) => {
                self.backend.set_user_error_msg(&format!("Invalid Pattern: {}", argument.clone()));
            }
        }
    }

    fn ci_insert_bytes(&mut self, bytes: Vec<u8>, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        for _ in 0 .. repeat {
            self.backend.insert_bytes_at_cursor(bytes.clone());
        }

        self.ci_cursor_right(bytes.len() * repeat);

        self.flag_row_update_by_offset(offset);
        self.frontend.raise_flag(Flag::UpdateOffset);
    }

    fn ci_overwrite_string(&mut self, argument: &str, repeat: usize) {
        match string_to_bytes(argument.to_string()) {
            Ok(converted_bytes) => {
                self.ci_overwrite_bytes(converted_bytes.clone(), repeat);
            }
            Err(_) => {
                self.backend.set_user_error_msg(&format!("Invalid Pattern: {}", argument.clone()));
            }
        }

    }

    fn ci_overwrite_bytes(&mut self, bytes: Vec<u8>, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        for _ in 0 .. repeat {
            self.backend.overwrite_bytes_at_cursor(bytes.clone());
            self.ci_cursor_right(bytes.len());
        }
        self.backend.set_cursor_length(1);


        self.frontend.raise_flag(Flag::CursorMoved);
        self.flag_row_update_by_range(offset..offset);
    }

    fn ci_increment(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        for _ in 0 .. repeat {
            match self.backend.increment_byte(offset) {
                Err(EditorError::OutOfRange(_, _)) => {
                    break;
                }
                Ok(_) => {}
            }
        }

        self.backend.set_cursor_length(1);

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

        self.flag_row_update_by_range(offset - suboffset .. offset);
        self.frontend.raise_flag(Flag::CursorMoved);
    }

    fn ci_decrement(&mut self, repeat: usize) {
        let offset = self.backend.get_cursor_offset();
        for _ in 0 .. repeat {
            match self.backend.decrement_byte(offset) {
                Ok(_) => {}
                Err(EditorError::OutOfRange(_, _)) => {
                    break;
                }
            }
        }
        self.backend.set_cursor_length(1);


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

        self.flag_row_update_by_range(offset - suboffset .. offset);
        self.frontend.raise_flag(Flag::CursorMoved);
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
                match self.backend.save() {
                    Ok(_) => {
                        match self.backend.get_active_file_path() {
                            Some(file_path) => {
                                self.backend.set_user_msg(&format!("Saved to file: {}", file_path));
                            }
                            None => () // Unreachable
                        }
                    }
                    Err(_e) => {
                        self.backend.set_user_error_msg("No path specified");
                    }
                }
            }
        }
    }

    fn ci_lock_viewport_width(&mut self, new_width: usize) {
        self.lock_viewport_width(new_width);

        self.frontend.raise_flag(Flag::SetupDisplays);
        self.frontend.raise_flag(Flag::RemapActiveRows);
    }

    fn ci_unlock_viewport_width(&mut self) {
        self.unlock_viewport_width();

        self.frontend.raise_flag(Flag::SetupDisplays);
        self.frontend.raise_flag(Flag::RemapActiveRows);
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

    fn flag_row_update_by_range(&mut self, range: std::ops::Range<usize>) {
        let (viewport_width, _) = self.backend.get_viewport_size();
        let first_active_row = range.start / viewport_width;
        let last_active_row = range.end / viewport_width;

        for y in first_active_row .. max(last_active_row + 1, first_active_row + 1) {
            self.frontend.raise_flag(Flag::UpdateRow(y));
        }
    }

    fn flag_row_update_by_offset(&mut self, offset: usize) {
        let (viewport_width, viewport_height) = self.backend.get_viewport_size();
        let first_active_row = offset / viewport_width;

        for y in first_active_row .. first_active_row + viewport_height {
            self.frontend.raise_flag(Flag::UpdateRow(y));
        }
    }
}
