pub mod inputter;
use super::editor::converter::ConverterError;
use inputter::*;
use inputter::function_ref::*;

pub trait Commandable {
    fn assign_line_command(&mut self, command_string: String, function: FunctionRef);
    fn try_command(&mut self, query: String);

    fn clear_register(&mut self);
    fn append_to_register(&mut self, new_digit: usize);
    fn grab_register(&mut self, default_if_unset: isize) -> isize;
    fn set_register_negative(&mut self);

    fn run_cmd_from_functionref(&mut self, funcref: FunctionRef, arguments: Vec<Vec<u8>>);
    fn string_to_bytes(&self, input_string: String) -> Result<Vec<u8>, ConverterError>;

    fn set_input_context(&mut self, context: u8);
}
