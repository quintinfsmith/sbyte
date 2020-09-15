use wrecked::{RectManager, logg};

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

    fn flag_row_update_by_offset(&mut self, offset: usize);
}

