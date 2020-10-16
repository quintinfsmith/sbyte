pub mod function_ref;

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

    pub fn read_input(&mut self, input_byte: u8) -> Option<(String, Vec<u8>)> {
        let mut output = None;

        self.input_buffer.push(input_byte);

        let input_buffer = self.input_buffer.clone();
        let mut clear_buffer = false;
        let mut new_context: Option<String>;
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
                        output = Some((funcref, self.input_buffer.clone()));
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
        let mut command_vec = command_string.as_bytes().to_vec();
        let mut mode_node = self.input_managers.entry(mode.to_string()).or_insert(InputNode::new());
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
        for (i, byte) in new_pattern.iter().enumerate() {
            tmp_pattern.push(*byte);
        }

        if tmp_pattern.len() > 0 {
            let next_byte = tmp_pattern.remove(0);

            let mut next_node = self.next_nodes.entry(next_byte).or_insert(InputNode::new());
            next_node.assign_command(tmp_pattern, hook);

        } else {
            self.hook = Some(hook.to_string());
        }
    }

    fn fetch_command(&mut self, input_pattern: Vec<u8>) -> (Option<String>, bool) {
        let mut output = (None, false);

        match &self.hook {
            Some(hook) => {
                // Found, Clear buffer
                output = (Some(hook.to_string()), true);
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
