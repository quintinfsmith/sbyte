use std::collections::HashMap;

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
    input_buffer: Vec<u8>
}

impl Inputter {
    pub fn new() -> Inputter {
        let mut output = Inputter {
            context: "".to_string(),
            killed: false,
            input_nodes: Vec::new(),
            active_node: 0,
            mode_roots: HashMap::new(),
            input_buffer: Vec::new()
        };
        output.set_context("DEFAULT");

        output
    }

    pub fn kill(&mut self) {
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

    pub fn input(&mut self, next_byte: u8) {
        self.input_buffer.push(next_byte);
    }

    fn path_continues(&mut self) -> bool {
        let mut output = false;

        if self.input_buffer.len() > 0 {
            let test_byte = self.input_buffer[0];
            match self.input_nodes.get(self.active_node) {
                Some(node) => {
                    match node.get_next(test_byte) {
                        Some(next_id) => {
                            output = true;
                        }
                        None => { }
                    }
                }
                None => { }
            }
        }

        output
    }

    /// Checks if the active node has a command hook AND that the input buffer won't continue
    fn hook_ready(&mut self) -> bool {
        let mut hook_result =match self.input_nodes.get(self.active_node) {
            Some(node) => {
                node.get_hook()
            }
            None => {
                None
            }
        };

        hook_result.is_some() && !self.path_continues()
    }

    pub fn fetch_hook(&mut self) -> Option<(String, Vec<String>)> {
        // Read in the input_buffer
        while self.input_buffer.len() > 0 && !self.hook_ready() {
            let working_byte = self.input_buffer.remove(0);
            let next = match self.input_nodes.get(self.active_node) {
                Some(input_node) => {
                    input_node.get_next(working_byte)
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

        // Then find the hook
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
