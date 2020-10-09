pub mod converter;
pub mod editor_cursor;
use converter::*;
use editor_cursor::*;


pub enum EditorError {
    OutOfRange
}

pub trait Editor {
    fn undo(&mut self);
    fn redo(&mut self);
    fn do_undo_or_redo(&mut self, task: (usize, usize, Vec<u8>)) -> (usize, usize, Vec<u8>);
    fn push_to_undo_stack(&mut self, offset: usize, bytes_to_remove: usize, bytes_to_insert: Vec<u8>);
    fn replace(&mut self, search_for: Vec<u8>, replace_with: Vec<u8>);
    fn make_selection(&mut self, offset: usize, length: usize);
    fn copy_to_clipboard(&mut self, bytes_to_copy: Vec<u8>);
    fn copy_selection(&mut self);
    fn get_clipboard(&mut self) -> Vec<u8>;
    fn load_file(&mut self, file_path: &str);
    fn save_as(&mut self, path: &str);
    fn save(&mut self);
    fn set_file_path(&mut self, new_file_path: String);
    fn find_all(&self, pattern: &Vec<u8>) -> Vec<usize>;
    fn find_after(&self, pattern: &Vec<u8>, offset: usize) -> Option<usize>;
    fn remove_bytes(&mut self, offset: usize, length: usize) -> Result<Vec<u8>, EditorError>;
    fn remove_bytes_at_cursor(&mut self) -> Result<Vec<u8>, EditorError>;
    fn insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) -> Result<(), EditorError>;
    fn insert_bytes_at_cursor(&mut self, new_bytes: Vec<u8>);
    fn overwrite_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) -> Result<Vec<u8>, EditorError>;
    fn overwrite_bytes_at_cursor(&mut self, new_bytes: Vec<u8>) -> Result<Vec<u8>, EditorError>;
    fn get_selected(&mut self) -> Vec<u8>;
    fn get_chunk(&mut self, offset: usize, length: usize) -> Vec<u8>;

    fn cursor_set_offset(&mut self, new_offset: usize);
    fn cursor_set_length(&mut self, new_length: isize);
    fn cursor_next_byte(&mut self);
    fn cursor_prev_byte(&mut self);
    fn cursor_increase_length(&mut self);
    fn cursor_decrease_length(&mut self);

    fn get_active_converter(&self) -> Box<dyn Converter>;
    fn get_display_ratio(&mut self) -> u8;

}
