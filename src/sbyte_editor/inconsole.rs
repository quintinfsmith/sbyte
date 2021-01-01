use wrecked::RectError;
use std::error::Error;
use super::Flag;


pub trait InConsole {
    fn flag_row_update_by_range(&mut self, range: std::ops::Range<usize>);
    fn flag_row_update_by_offset(&mut self, offset: usize);

    fn check_flag(&mut self, key: Flag) -> bool;
    fn raise_flag(&mut self, key: Flag);
    fn lower_flag(&mut self, key: Flag);
    fn raise_row_update_flag(&mut self, absolute_y: usize);

    fn lock_viewport_width(&mut self, new_width: usize);
    fn unlock_viewport_width(&mut self);
}
