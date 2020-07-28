pub mod inputter;
use inputter::*;
use inputter::function_ref::*;

pub trait Commandable {
    fn assign_line_command(&mut self, command_string: String, function: FunctionRef);
    fn try_command(&mut self, query: String);

    fn clear_register(&mut self);
    fn append_to_register(&mut self, new_digit: isize);
    fn grab_register(&mut self, default_if_unset: isize) -> isize;

    fn run_cmd_from_functionref(&mut self, funcref: FunctionRef, arguments: Vec<String>);
    fn string_to_bytes(&mut self, input_string: String) -> Vec<u8>;
}
