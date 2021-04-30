use std::collections::HashMap;
use std::cmp::{min, max};
type R = Result<(), SbyteError>;
type Callback = fn(&mut Shell, &[&str]) -> R;

use super::editor::{Editor, SbyteError, string_to_bytes};
pub struct Shell {
    hook_map: HashMap<String, Callback>,
    alias_map: HashMap<String, String>,
    editor: Editor,
    query_buffer: Option<String>,
    buffer_history: Vec<String>,
    feedback: Option<String>,
    error: Option<String>,
    register: Option<usize>
}

impl Shell {
    pub fn new() -> Shell {
        let mut output = Shell {
            editor: Editor::new(),
            hook_map: HashMap::new(),
            alias_map: HashMap::new(),
            query_buffer: None,
            buffer_history: Vec::new(),
            feedback: None,
            error: None,
            register: None
        };

        output.map_command("TOGGLE_CONVERTER", hook_toggle_converter);
        output.map_command("QUIT", hook_send_kill_signal);

        output.map_command("CURSOR_UP", hook_cursor_up);
        output.map_command("CURSOR_DOWN", hook_cursor_down);
        output.map_command("CURSOR_LEFT", hook_cursor_left);
        output.map_command("CURSOR_RIGHT", hook_cursor_right);

        output.map_command("SUBCURSOR_LEFT", hook_subcursor_left);
        output.map_command("SUBCURSOR_RIGHT", hook_subcursor_right);

        output.map_command("CURSOR_LENGTH_DOWN", hook_cursor_length_down);
        output.map_command("CURSOR_LENGTH_UP", hook_cursor_length_up);
        output.map_command("CURSOR_LENGTH_LEFT", hook_cursor_length_left);
        output.map_command("CURSOR_LENGTH_RIGHT", hook_cursor_length_right);

        output.map_command("JUMP_TO_REGISTER", hook_jump_to_position);
        output.map_command("JUMP_TO_NEXT_HIGHLIGHTED", hook_jump_to_next_selection);
        output.map_command("JUMP_TO_PREVIOUS_HIGHLIGHTED", hook_jump_to_previous_selection);
        output.map_command("JUMP_TO_PATTERN", hook_jump_to_pattern);
        output.map_command("POINTER_BE_JUMP", hook_jump_big_endian);
        output.map_command("POINTER_LE_JUMP", hook_jump_little_endian);

        output.map_command("BACKSPACE", hook_backspace);
        output.map_command("DELETE", hook_delete);
        output.map_command("YANK", hook_yank);
        output.map_command("PASTE", hook_paste);
        output.map_command("UNDO", hook_undo);
        output.map_command("REDO", hook_redo);
        output.map_command("CLEAR_REGISTER", hook_clear_register);
        output.map_command("APPEND_TO_REGISTER", hook_push_to_register);
        output.map_command("INCREMENT", hook_increment);
        output.map_command("DECREMENT", hook_decrement);
        output.map_command("OVERWRITE_DIGIT", hook_overwrite_digit);
        output.map_command("INSERT_STRING", hook_insert_string);
        output.map_command("OVERWRITE_STRING", hook_overwrite_string);
        output.map_command("BITWISE_NOT", hook_bitwise_not);
        output.map_command("BITWISE_AND", hook_bitwise_and_mask);
        output.map_command("BITWISE_NAND", hook_bitwise_nand_mask);
        output.map_command("BITWISE_NOR", hook_bitwise_nor_mask);
        output.map_command("BITWISE_OR", hook_bitwise_or_mask);
        output.map_command("BITWISE_XOR", hook_bitwise_xor_mask);

        output.map_command("APPEND_TO_COMMANDLINE", hook_push_to_buffer);
        output.map_command("CMDLINE_BACKSPACE", hook_pop_from_buffer);
        output.map_command("RUN_CUSTOM_COMMAND", hook_query);
        output.map_command("REPLACE_ALL", hook_replace_pattern);

        output.map_command("ALIAS", hook_set_alias);

        output.map_command("SAVE", hook_save);
        output.map_command("SAVEQUIT", hook_save_quit);

        output.map_alias("q", "QUIT");
        output.map_alias("w", "SAVE");
        output.map_alias("wq", "SAVEQUIT");
        output.map_alias("find", "JUMP_TO_NEXT");
        output.map_alias("fr", "REPLACE_ALL");
        output.map_alias("insert", "INSERT_STRING");
        output.map_alias("overwrite", "OVERWRITE");

        output.map_alias("and", "MASK_AND");
        output.map_alias("nand", "MASK_NAND");
        output.map_alias("or", "MASK_OR");
        output.map_alias("nor", "MASK_NOR");
        output.map_alias("xor", "MASK_XOR");
        output.map_alias("not", "BITWISE_NOT");
        output.map_alias("rep", "REPLACE_ALL");

       // output.map_command("", );

        output
    }

    pub fn buffer_clear(&mut self) {
        self.query_buffer = None;
    }

    pub fn buffer_get(&self) -> Option<String> {
        self.query_buffer.clone()
    }

    pub fn buffer_fetch(&mut self) -> Option<String> {
        let output = self.query_buffer.clone();
        self.buffer_clear();

        match output {
            Some(ref buffer) => {
                self.buffer_history.push(buffer.clone());
            }
            None => ()
        }

        output
    }

    pub fn buffer_push(&mut self, input: &str) {
        let working_string;
        match &self.query_buffer {
            Some(buffer) => {
                working_string = format!("{}{}", buffer, input);
            }
            None => {
                working_string = input.to_string();
            }
        }
        self.query_buffer = Some(working_string);
    }

    pub fn buffer_pop(&mut self) -> Option<char> {
        if self.query_buffer.is_some() {
            let mut c = self.query_buffer.as_deref_mut().unwrap().chars();
            let output = c.next_back();
            let tmp = c.as_str().to_string();

            if output.is_some() {
                self.query_buffer = Some(tmp);
            } else {
                self.query_buffer = None;
            }

            output
        } else {
            None
        }
    }

    pub fn query(&mut self) -> R {
        match self.buffer_fetch() {
            Some(buffer) => {
                let mut words = parse_words(&buffer);
                if words.len() > 0 {
                    let cmd = words.remove(0);
                    let mut args = vec![];
                    for word in words.iter() {
                        args.push(word.as_str());
                    }
                    self.try_command(&cmd, args.as_slice())
                } else {
                   Err(SbyteError::InvalidCommand(buffer.to_string()))
                }
            }
            None => {
                Err(SbyteError::NoCommandGiven)
            }
        }
    }

    pub fn register_clear(&mut self) {
        self.register = None;
    }

    pub fn register_get(&self) -> Option<usize> {
        self.register.clone()
    }

    pub fn register_fetch(&mut self, default_if_unset: usize) -> usize {
        let output = match self.register {
            Some(n) => {
                n
            }
            None => {
                default_if_unset
            }
        };
        self.register_clear();

        output
    }

    pub fn register_push(&mut self, new_digit: usize) {
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

    fn map_command(&mut self, key: &str, function: Callback) {
        self.hook_map.insert(key.to_string(), function);
    }

    fn map_alias(&mut self, alias: &str, command_key: &str) -> R {
        if self.hook_map.contains_key(&command_key.to_string()) {
            self.alias_map.insert(alias.to_string(), command_key.to_string());
            Ok(())
        } else {
            Err(SbyteError::InvalidCommand(command_key.to_string()))
        }
    }

    pub fn try_command(&mut self, key: &str, args: &[&str]) -> Result<(), SbyteError> {
        let mut use_key = key;
        if ! self.hook_map.contains_key(&key.to_string()) {
            match self.alias_map.get(&key.to_string()) {
                Some(real_key) => {
                    use_key = real_key.as_str();
                }
                None => { }
            }
        }

        match self.hook_map.get(use_key) {
            Some(f) => {
                f(self, args)
            }
            None => {
                self.log_error(&format!("Invalid Command: \"{}\"", use_key));
                Ok(())
            }
        }
    }

    fn handle_result(&mut self, result: Result<(), SbyteError>) {
        // TODO
    }

    pub fn get_editor(&self) -> &Editor {
        &self.editor
    }
    pub fn get_editor_mut(&mut self) -> &mut Editor {
        &mut self.editor
    }

    pub fn fetch_feedback(&mut self) -> Option<String> {
        let output = self.feedback.clone();
        self.feedback = None;
        output
    }
    pub fn fetch_error(&mut self) -> Option<String> {
        let output = self.error.clone();
        self.error = None;
        output
    }

    pub fn log_error(&mut self, msg: &str) {
        self.error = Some(msg.to_string())
    }
    pub fn log_feedback(&mut self, msg: &str) {
        self.feedback = Some(msg.to_string())
    }
}

//////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////

fn hook_clear_register(shell: &mut Shell, args: &[&str]) -> R {
    shell.register_clear();
    Ok(())
}

fn hook_set_register(shell: &mut Shell, args: &[&str]) -> R {
    shell.register_clear();
    hook_push_to_register(shell, args)
}

fn hook_push_to_register(shell: &mut Shell, args: &[&str]) -> R {
    let mut digit;
    for arg in args.iter() {
        for c in arg.chars() {
            if c.is_digit(10) {
                digit = c.to_digit(10).unwrap() as usize;
                shell.register_push(digit);
            } else {
                // TODO Throw Error
            }
        }
    }

    match shell.register_get() {
        Some(register) => {
            shell.log_feedback(&format!("[{}]", register))
        }
        None => { }
    }

    Ok(())
}

fn hook_clear_buffer(shell: &mut Shell, args: &[&str]) -> R {
    shell.buffer_clear();
    Ok(())
}

fn hook_push_to_buffer(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        shell.buffer_push(arg);
    }
    Ok(())
}

fn hook_pop_from_buffer(shell: &mut Shell, args: &[&str]) -> R {
    match shell.buffer_pop() {
        Some(_) => {
            Ok(())
        }
        None => {
            Err(SbyteError::BufferEmpty)
        }
    }
}

fn hook_query(shell: &mut Shell, args: &[&str]) -> R {
    shell.query()
}


//fn(shell: &mut Shell, args: &[&str]) -> R {
////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

fn hook_cursor_up(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    shell.get_editor_mut().set_cursor_length(1);

    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_prev_line();
    }

    Ok(())
}

fn hook_cursor_down(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    shell.get_editor_mut().set_cursor_length(1);

    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_next_line();
    }

    Ok(())
}

fn hook_cursor_left(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    shell.get_editor_mut().set_cursor_length(1);

    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_prev_byte();
    }

    Ok(())
}

fn hook_cursor_right(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    shell.get_editor_mut().set_cursor_length(1);

    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_next_byte();
    }

    Ok(())
}

fn hook_cursor_length_up(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_decrease_length_by_line();
    }

    Ok(())
}

fn hook_cursor_length_down(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_increase_length_by_line();
    }

    Ok(())
}

fn hook_cursor_length_left(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_decrease_length();
    }

    Ok(())
}

fn hook_cursor_length_right(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = shell.register_fetch(1);
    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_increase_length();
    }

    Ok(())
}

fn hook_subcursor_left(shell: &mut Shell, args: &[&str]) -> R {
    shell.get_editor_mut().subcursor_prev_digit();
    Ok(())
}

fn hook_subcursor_right(shell: &mut Shell, args: &[&str]) -> R {
    shell.get_editor_mut().subcursor_next_digit();
    Ok(())
}

fn hook_replace_pattern(shell: &mut Shell, args: &[&str]) -> R {
    if args.len() >= 2 {
        match string_to_bytes(args[1]) {
            Ok(replacer) => {
                match shell.get_editor_mut().replace(args[0], &replacer) {
                    Ok(indeces) => {
                        if indeces.len() == 0 {
                            shell.log_error(&format!("Pattern \"{}\" not found", args[0]));
                        } else {
                            shell.log_feedback(&format!("Replaced {} instances", indeces.len()));
                        }
                    }
                    Err(e) => {
                        shell.log_error(&format!("{:?}", e));
                        Err(e)?;
                    }
                }
            }
            Err(e) => {
                shell.log_error(&format!("{:?}", e));
            }
        }
    }

    Ok(())
}

fn hook_overwrite_digit(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        for c in arg.chars() {
            shell.get_editor_mut().replace_digit(c)?;
            shell.get_editor_mut().subcursor_next_digit();
            if shell.get_editor_mut().get_subcursor_offset() == 0
            && shell.get_editor_mut().get_cursor_length() == 1 {
                shell.get_editor_mut().cursor_next_byte();
            }
        }
    }
    Ok(())
}

fn hook_jump_to_position(shell: &mut Shell, args: &[&str]) -> R {
    let default = shell.get_editor().len();
    let new_offset = shell.register_fetch(default);
    shell.get_editor_mut().set_cursor_length(1);
    shell.get_editor_mut().set_cursor_offset(new_offset)?;

    Ok(())
}

fn hook_jump_big_endian(shell: &mut Shell, args: &[&str]) -> R {
    let new_offset = shell.get_editor_mut().get_selected_as_big_endian();
    shell.get_editor_mut().set_cursor_length(1);
    shell.get_editor_mut().set_cursor_offset(new_offset)?;

    Ok(())
}

fn hook_jump_little_endian(shell: &mut Shell, args: &[&str]) -> R {
    let new_offset = shell.get_editor_mut().get_selected_as_little_endian();
    shell.get_editor_mut().set_cursor_length(1);
    shell.get_editor_mut().set_cursor_offset(new_offset)?;

    Ok(())
}

fn hook_jump_register(shell: &mut Shell, args: &[&str]) -> R {
    let default = shell.get_editor().len();
    let new_offset = shell.register_fetch(default);

    shell.get_editor_mut().set_cursor_length(1);
    shell.get_editor_mut().set_cursor_offset(new_offset)?;

    Ok(())
}



fn hook_bitwise_not(shell: &mut Shell, args: &[&str]) -> R {
    shell.get_editor_mut().bitwise_not()
}

fn hook_bitwise_nor_mask(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        match string_to_bytes(arg) {
            Ok(mask) => {
                shell.get_editor_mut().apply_nor_mask(&mask)?;
            }
            Err(_e) => {
                //TODO: Handle this
            }
        }
    }

    Ok(())
}

fn hook_bitwise_and_mask(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        match string_to_bytes(arg) {
            Ok(mask) => {
                shell.get_editor_mut().apply_and_mask(&mask)?;
            }
            Err(_e) => {
                //TODO: Handle this
            }
        }
    }
    Ok(())
}

fn hook_bitwise_nand_mask(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        match string_to_bytes(arg) {
            Ok(mask) => {
                shell.get_editor_mut().apply_nand_mask(&mask)?;
            }
            Err(_e) => {
                //TODO: Handle this
            }
        }
    }
    Ok(())
}

fn hook_bitwise_or_mask(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        match string_to_bytes(arg) {
            Ok(mask) => {
                shell.get_editor_mut().apply_or_mask(&mask)?;
            }
            Err(_e) => {
                //TODO: Handle this
            }
        }
    }
    Ok(())
}

fn hook_bitwise_xor_mask(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        match string_to_bytes(arg) {
            Ok(mask) => {
                shell.get_editor_mut().apply_xor_mask(&mask)?;
            }
            Err(_e) => {
                //TODO: Handle this
            }
        }
    }

    Ok(())
}

fn hook_yank(shell: &mut Shell, args: &[&str]) -> R {
    shell.get_editor_mut().copy_selection();
    shell.get_editor_mut().set_cursor_length(1);

    let clipboard_len = shell.get_editor().get_clipboard().len();
    shell.log_feedback(&format!("Yanked {} bytes", clipboard_len));
    Ok(())
}


fn hook_paste(shell: &mut Shell, args: &[&str]) -> R {
    let to_paste = shell.get_editor_mut().get_clipboard();

    for _ in 0 .. shell.register_fetch(1) {
        shell.get_editor_mut().insert_bytes_at_cursor(&to_paste)?;
        for i in 0 .. to_paste.len() {
            shell.get_editor_mut().cursor_next_byte();
        }
    }

    Ok(())
}

fn hook_delete(shell: &mut Shell, args: &[&str]) -> R {
    let mut removed_bytes = Vec::new();
    for _ in 0 .. shell.register_fetch(1) {
        removed_bytes.extend(shell.get_editor_mut().remove_bytes_at_cursor().iter().copied());
    }
    shell.get_editor_mut().copy_to_clipboard(removed_bytes);
    shell.get_editor_mut().set_cursor_length(1);

    let clipboard_len = shell.get_editor().get_clipboard().len();
    shell.log_feedback(&format!("{} fewer bytes", clipboard_len));

    Ok(())
}

fn hook_backspace(shell: &mut Shell, args: &[&str]) -> R {
    let repeat = min(shell.register_fetch(1), shell.get_editor_mut().get_cursor_offset());
    for _ in 0 .. repeat {
        shell.get_editor_mut().cursor_prev_byte();
    }

    shell.get_editor_mut().set_cursor_length(repeat as isize);

    let mut removed_bytes = Vec::new();
    for _ in 0 .. repeat {
        removed_bytes.extend(shell.get_editor_mut().remove_bytes_at_cursor().iter().copied());
    }

    shell.get_editor_mut().copy_to_clipboard(removed_bytes);
    shell.get_editor_mut().set_cursor_length(1);

    Ok(())

}

fn hook_undo(shell: &mut Shell, args: &[&str]) -> R {
    for i in 0 .. shell.register_fetch(1) {
        match shell.get_editor_mut().undo() {
            Ok(_) => {
                shell.log_feedback(&format!("Undid {} actions", i + 1));
            }
            Err(SbyteError::EmptyStack) => {
                if i == 0 {
                    shell.log_error("Nothing to undo");
                }
                break;
            }
            Err(e) => {
                Err(e)?;
            }
        }
    }

    Ok(())
}

fn hook_redo(shell: &mut Shell, args: &[&str]) -> R {
    for i in 0 .. shell.register_fetch(1) {
        match shell.get_editor_mut().redo() {
            Ok(_) => {
                shell.log_feedback(&format!("Redid {} actions", i + 1));
            }
            Err(SbyteError::EmptyStack) => {
                if i == 0 {
                    shell.log_error("Nothing to do");
                }
                break;
            }
            Err(e) => {
                Err(e)?;
            }
        }
    }

    Ok(())
}

fn hook_insert_string(shell: &mut Shell, args: &[&str]) -> R {
    for i in 0 .. shell.register_fetch(1) {
        for arg in args.iter() {
            match string_to_bytes(arg) {
                Ok(converted) => {
                    shell.get_editor_mut().set_cursor_length(1);
                    shell.get_editor_mut().insert_bytes_at_cursor(&converted)?;
                    for _ in 0 .. converted.len() {
                        shell.get_editor_mut().cursor_next_byte();
                    }
                }
                Err(_e) => ()
            }

        }
    }

    Ok(())
}

fn hook_overwrite_string(shell: &mut Shell, args: &[&str]) -> R {
    for i in 0 .. shell.register_fetch(1) {
        for arg in args.iter() {
            match string_to_bytes(arg) {
                Ok(converted) => {
                    shell.get_editor_mut().set_cursor_length(1);
                    shell.get_editor_mut().overwrite_bytes_at_cursor(&converted)?;
                    for _ in 0 .. converted.len() {
                        shell.get_editor_mut().cursor_next_byte();
                    }
                }
                Err(_e) => ()
            }

        }
    }

    Ok(())
}

fn hook_increment(shell: &mut Shell, args: &[&str]) -> R {
    let offset = shell.get_editor_mut().get_cursor_offset();
    let cursor_length = shell.get_editor_mut().get_cursor_length();
    let repeat = shell.register_fetch(1);
    for _ in 0 .. repeat {
        match shell.get_editor_mut().increment_byte(offset + (cursor_length - 1), cursor_length) {
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
        chunk = shell.get_editor_mut().get_chunk(offset - suboffset, 1);
        if chunk.len() > 0 && (chunk[0] as u32) < (repeat >> (8 * suboffset)) as u32 {
            suboffset += 1;
        } else {
            break;
        }
    }

    Ok(())
}

fn hook_decrement(shell: &mut Shell, args: &[&str]) -> R {
    let offset = shell.get_editor_mut().get_cursor_offset();
    let cursor_length = shell.get_editor_mut().get_cursor_length();
    let repeat = shell.register_fetch(1);
    for _ in 0 .. repeat {
        match shell.get_editor_mut().decrement_byte(offset + (cursor_length - 1), cursor_length) {
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
        chunk = shell.get_editor_mut().get_chunk(offset - suboffset, 1);
        if chunk.len() > 0 && (chunk[0] as u32) < (repeat >> (8 * suboffset)) as u32 {
            suboffset += 1;
        } else {
            break;
        }
    }

    Ok(())
}


fn hook_save(shell: &mut Shell, args: &[&str]) -> R {
    if !args.is_empty() {
        for arg in args.iter() {
        // TODO: Handle Result here
            shell.get_editor_mut().save_as(arg);
        }
    } else {
        // TODO: Handle Result here
        shell.get_editor_mut().save();
    }

    Ok(())
}

fn hook_save_quit(shell: &mut Shell, args: &[&str]) -> R {
    hook_save(shell, args)?;
    Err(SbyteError::KillSignal)
}

fn hook_toggle_converter(shell: &mut Shell, args: &[&str]) -> R {
    shell.get_editor_mut().toggle_converter();
    Ok(())
}

fn hook_jump_to_previous_selection(shell: &mut Shell, args: &[&str]) -> R {
    let selection = shell.get_editor_mut().get_selected();
    let mut string_rep = "".to_string();
    for ord in selection.iter() {
        string_rep = format!("{}\\x{:X}{:X}", string_rep, ord >> 4, ord & 0x0F);
    }

    jump_to_previous(shell, Some(&string_rep))
}

fn hook_jump_to_next_selection(shell: &mut Shell, args: &[&str]) -> R {
    let selection = shell.get_editor_mut().get_selected();
    let mut string_rep = "".to_string();
    for ord in selection.iter() {
        string_rep = format!("{}\\x{:X}{:X}", string_rep, ord >> 4, ord & 0x0F);
    }

    jump_to_next(shell, Some(&string_rep))
}


fn hook_send_kill_signal(_: &mut Shell, _: &[&str]) -> R {
    Err(SbyteError::KillSignal)
}


fn hook_jump_to_pattern(shell: &mut Shell, args: &[&str]) -> R {
    for arg in args.iter() {
        jump_to_next(shell, Some(arg))?;
    }

    Ok(())
}

fn hook_set_alias(shell: &mut Shell, args: &[&str]) -> R {
    if args.len() >= 2 {
        match shell.map_alias(args[0], args[1]) {
            Ok(_) => { }
            Err(SbyteError::InvalidCommand(_)) => {
                shell.log_error(&format!("Invalid Command: \"{}\"", args[1]));
            }
            Err(e) => {
                Err(e)?;
            }
        }
    } else {
        shell.log_error("Alias and command key required");
    }

    Ok(())
}
////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////

fn jump_to_previous(shell: &mut Shell, argument: Option<&str>) -> R {
    let current_offset = shell.get_editor_mut().get_cursor_offset();
    let option_pattern: Option<String> = match argument {
        Some(pattern) => { // arugment was given. use it.
            Some(pattern.to_string())
        }
        None => {
            match shell.get_editor_mut().get_search_history().last() {
                Some(byte_pattern) => {
                    Some(byte_pattern.clone())
                }
                None => {
                    None
                }
            }
        }
    };

    let repeat = shell.register_fetch(0);


    // TODO: Feedback
    match option_pattern {
        Some(string_rep) => {
            shell.get_editor_mut().add_search_history(string_rep.clone());
            match shell.get_editor_mut().find_nth_before(&string_rep, current_offset, repeat) {
                Ok(result) => {
                    match result {
                        Some(new_offset) => {
                            shell.get_editor_mut().set_cursor_length((new_offset.1 - new_offset.0) as isize);
                            shell.get_editor_mut().set_cursor_offset(new_offset.0);
                        }
                        None => ()
                    }
                }
                Err(_e) => ()
            }
        }
        None => ()
    }

    Ok(())
}

fn jump_to_next(shell: &mut Shell, argument: Option<&str>) -> R {
    let current_offset = shell.get_editor_mut().get_cursor_offset();
    let option_pattern: Option<String> = match argument {
        Some(pattern) => { // arugment was given. use it.
            Some(pattern.to_string())
        }
        None => {
            match shell.get_editor_mut().get_search_history().last() {
                Some(byte_pattern) => {
                    Some(byte_pattern.clone())
                }
                None => {
                    None
                }
            }
        }
    };

    let repeat = shell.register_fetch(0);

    // TODO: Feedback
    match option_pattern {
        Some(string_rep) => {
            shell.get_editor_mut().add_search_history(string_rep.clone());
            match shell.get_editor_mut().find_nth_after(&string_rep, current_offset, repeat) {
                Ok(result) => {
                    match result {
                        Some(new_offset) => {
                            shell.get_editor_mut().set_cursor_length((new_offset.1 - new_offset.0) as isize);
                            shell.get_editor_mut().set_cursor_offset(new_offset.0);
                        }
                        None => ()
                    }
                }
                Err(_e) => ()
            }
        }
        None => ()
    }

    Ok(())
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
