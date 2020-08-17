use wrecked::{RectManager, logg};

pub trait InConsole {
    fn tick(&mut self);

    fn check_resize(&mut self);
    fn setup_displays(&mut self);
    fn apply_cursor(&mut self);
    fn remove_cursor(&mut self);

    fn remap_active_rows(&mut self);

    fn set_row_characters(&mut self, offset: usize);
    fn autoset_viewport_size(&mut self);

    fn set_offset_display(&mut self);
    fn arrange_displays(&mut self);
    fn display_user_message(&mut self);

    fn draw_cmdline(&mut self);
}

