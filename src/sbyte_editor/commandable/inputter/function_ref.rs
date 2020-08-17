#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionRef {
    NULL,
    CURSOR_UP,
    CURSOR_DOWN,
    CURSOR_LEFT,
    CURSOR_RIGHT,
    CURSOR_LENGTH_UP,
    CURSOR_LENGTH_DOWN,
    CURSOR_LENGTH_LEFT,
    CURSOR_LENGTH_RIGHT,
    INSERT,
    OVERWRITE,
    DELETE,
    BACKSPACE,
    APPEND_TO_REGISTER,
    JUMP_TO_REGISTER,
    CLEAR_REGISTER,
    UNDO,
    REDO,
    JUMP_TO_NEXT,

    INSERT_TO_CMDLINE,
    CMDLINE_BACKSPACE,
    RUN_CUSTOM_COMMAND,

    TOGGLE_CONVERTER,

    // MODE_SET are grouped with the main at the top, then aliases proceeding.
    // ie MODE_SET_APPEND is an alias of MODE_SET_INSERT, but will move the cursor first

    MODE_SET_MOVE,

    MODE_SET_INSERT,
    MODE_SET_APPEND,

    MODE_SET_OVERWRITE,

    MODE_SET_CMD,
    MODE_SET_SEARCH,
    MODE_SET_INSERT_SPECIAL,
    MODE_SET_OVERWRITE_SPECIAL,


    KILL,
    SAVE,
    SAVEKILL
}