pub mod function_ref;

use function_ref::FunctionRef;
use std::collections::HashMap;

pub struct Inputter {
    input_managers: HashMap<u8, InputNode>,
    input_buffer: Vec<u8>,
    context: u8,
    context_keys: HashMap<FunctionRef, u8>
}

impl Inputter {
    pub fn new() -> Inputter {
        Inputter {
            input_managers: HashMap::new(),
            input_buffer: Vec::new(),
            context: 0,
            context_keys: HashMap::new()
        }
    }

    pub fn read_input(&mut self, input_byte: u8) -> Option<(FunctionRef, u8)> {
        let mut output = None;

        self.input_buffer.push(input_byte);

        let input_buffer = self.input_buffer.clone();
        let mut clear_buffer = false;
        let mut new_context = self.context;
        match self.input_managers.get_mut(&self.context) {
            Some(root_node) => {
                let (cmd, completed_path) = root_node.fetch_command(input_buffer);
                match cmd {
                    Some(funcref) => {
                        match self.context_keys.get(&funcref) {
                            Some(_new_context) => {
                                new_context = *_new_context;
                            }
                            None => ()
                        };
                        output = Some((funcref, input_byte));
                    }
                    None => ()
                }
                clear_buffer = completed_path;
            }
            None => ()
        }

        self.context = new_context;

        if (clear_buffer) {
            self.input_buffer.drain(..);
        }

        output
    }

    pub fn assign_mode_command(&mut self, mode: u8, command_string: String, hook: FunctionRef) {
        let mut command_vec = command_string.as_bytes().to_vec();
        let mut mode_node = self.input_managers.entry(mode).or_insert(InputNode::new());
        mode_node.assign_command(command_vec, hook);
    }

    pub fn set_context_key(&mut self, funcref: FunctionRef, mode: u8) {
        self.context_keys.entry(funcref)
            .and_modify(|e| { *e = mode })
            .or_insert(mode);
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