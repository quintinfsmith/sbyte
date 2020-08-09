pub mod viewport;
use viewport::*;

pub trait VisualEditor {
    fn cursor_next_line(&mut self);
    fn cursor_prev_line(&mut self);
    fn cursor_increase_length_by_line(&mut self);
    fn cursor_decrease_length_by_line(&mut self);
    fn adjust_viewport_offset(&mut self);
}
