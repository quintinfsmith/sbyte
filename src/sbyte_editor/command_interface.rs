pub trait CommandInterface {
    //fn ci_assign_input(&mut self, funcref: &str, sequence: &str);
    fn ci_cursor_up(&mut self, repeat: usize);
    fn ci_cursor_down(&mut self, repeat: usize);
    fn ci_cursor_left(&mut self, repeat: usize);
    fn ci_cursor_right(&mut self, repeat: usize);

    fn ci_cursor_length_up(&mut self, repeat: usize);
    fn ci_cursor_length_down(&mut self, repeat: usize);
    fn ci_cursor_length_left(&mut self, repeat: usize);
    fn ci_cursor_length_right(&mut self, repeat: usize);

    fn ci_jump_to_next(&mut self, pattern: Option<Vec<u8>>, repeat: usize);
    fn ci_delete(&mut self, repeat: usize);
    fn ci_backspace(&mut self, repeat: usize);
    fn ci_undo(&mut self, repeat: usize);
    fn ci_redo(&mut self, repeat: usize);

    fn ci_insert_string(&mut self, string: &str, repeat: usize);
    fn ci_insert_bytes(&mut self, bytes: Vec<u8>, repeat: usize);

    fn ci_overwrite_string(&mut self, string: &str, repeat: usize);
    fn ci_overwrite_bytes(&mut self, bytes: Vec<u8>, repeat: usize);
    fn ci_decrement(&mut self, repeat: usize);
    fn ci_increment(&mut self, repeat: usize);

    fn ci_jump_to_position(&mut self, new_position: usize);

    fn ci_yank(&mut self);

    fn ci_save(&mut self, path: Option<&str>);

    fn ci_lock_viewport_width(&mut self, new_width: usize);
    fn ci_unlock_viewport_width(&mut self);

    //fn ci_remove_structure(&mut self);
    //fn ci_create_big_endian_structure(&mut self);
    //fn ci_set_register(&mut self, new_value: usize);
    //fn ci_append_to_register(&mut self, next_dec_digit: u8);
    //fn ci_clear_register(&mut self);

    //*ci_ Maybe not use the following *here*
    //fn ci_cmdline_backspace(&mut self);
    //fn ci_insert_to_cmdline(&mut self, string: String);
    //fn ci_run_custom_command(&mut self);

}
