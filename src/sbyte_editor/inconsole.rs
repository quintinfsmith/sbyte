use wrecked::{RectManager, logg};

#[derive(Hash, PartialEq, Eq)]
pub enum Flag {
    CURSOR_MOVED,
    FULL_REFRESH,
    DISPLAY_REFRESH,
    SETUP_DISPLAYS,
    REMAP_ACTIVE_ROWS,
    UPDATE_OFFSET,
    DISPLAY_CMDLINE,
    UPDATE_ROW(usize)
}

pub enum FlagError {
    NotFound
}

pub trait InConsole {
    fn tick(&mut self);

    fn check_resize(&mut self);
    fn setup_displays(&mut self);
    fn apply_cursor(&mut self);

    fn remap_active_rows(&mut self);

    fn set_row_characters(&mut self, offset: usize);
    fn autoset_viewport_size(&mut self);

    fn arrange_displays(&mut self);

    fn display_user_offset(&mut self);
    fn display_user_message(&mut self, msg: String);
    fn display_user_error(&mut self, msg: String);
    fn display_command_line(&mut self);

    fn clear_meta_rect(&mut self);

    fn flag_row_update_by_range(&mut self, range: std::ops::Range<usize>);
    fn flag_row_update_by_offset(&mut self, offset: usize);

    fn check_flag(&mut self, key: Flag) -> bool;
    fn raise_flag(&mut self, key: Flag);
    fn lower_flag(&mut self, key: Flag);
    fn raise_row_update_flag(&mut self, absolute_y: usize);

    fn lock_viewport_width(&mut self, new_width: usize);
    fn unlock_viewport_width(&mut self);
}

