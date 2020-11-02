use wrecked::RectError;
use std::error::Error;

#[derive(Hash, PartialEq, Eq)]
pub enum Flag {
    CursorMoved,
    FullRefresh,
    DisplayRefresh,
    SetupDisplays,
    RemapActiveRows,
    UpdateOffset,
    DisplayCMDLine,
    UpdateRow(usize)
}

pub enum FlagError {
    NotFound
}

pub trait InConsole {
    fn tick(&mut self) -> Result<(), Box<dyn Error>>;

    fn check_resize(&mut self);
    fn setup_displays(&mut self) -> Result<(), RectError>;
    fn apply_cursor(&mut self) -> Result<(), RectError>;

    fn remap_active_rows(&mut self) -> Result<(), RectError>;

    fn set_row_characters(&mut self, offset: usize) -> Result<(), RectError>;
    fn autoset_viewport_size(&mut self);

    fn arrange_displays(&mut self) -> Result<(), RectError>;

    fn display_user_offset(&mut self) -> Result<(), RectError>;
    fn display_user_message(&mut self, msg: String) -> Result<(), RectError>;
    fn display_user_error(&mut self, msg: String) -> Result<(), RectError>;
    fn display_command_line(&mut self) -> Result<(), RectError>;

    fn clear_meta_rect(&mut self) -> Result<(), RectError>;

    fn flag_row_update_by_range(&mut self, range: std::ops::Range<usize>);
    fn flag_row_update_by_offset(&mut self, offset: usize);

    fn check_flag(&mut self, key: Flag) -> bool;
    fn raise_flag(&mut self, key: Flag);
    fn lower_flag(&mut self, key: Flag);
    fn raise_row_update_flag(&mut self, absolute_y: usize);

    fn lock_viewport_width(&mut self, new_width: usize);
    fn unlock_viewport_width(&mut self);
}
