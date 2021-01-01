pub struct InputterEditorInterface {
    function_queue: Vec<(String, String)>,

    new_context: Option<String>,
    new_input_sequences: Vec<(String, String, String)>,

    flag_kill: bool
}

impl InputterEditorInterface {
    pub fn new() -> InputterEditorInterface {
        InputterEditorInterface {
            function_queue: Vec::new(),

            new_context: None,
            new_input_sequences: Vec::new(),

            flag_kill: false
        }
    }
}


