pub mod inputter;
use super::editor::converter::ConverterError;
use inputter::*;

pub trait Commandable {
    fn assign_line_command(&mut self, command_string: &str, function: &str);
    fn try_command(&mut self, query: &str);

    fn clear_register(&mut self);
    fn append_to_register(&mut self, new_digit: usize);
    fn grab_register(&mut self, default_if_unset: usize) -> usize;

    fn run_cmd_from_functionref(&mut self, funcref: &str, arguments: Vec<Vec<u8>>);
    fn string_to_bytes(&self, input_string: String) -> Result<Vec<u8>, ConverterError>;
    fn string_to_integer(&self, input_string: &str) -> Result<usize, ConverterError>;

    fn set_input_context(&mut self, context: &str);
}
