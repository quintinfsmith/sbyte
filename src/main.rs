use std::env;
use std::path::Path;
use std::error::Error;
pub mod sbyte_editor;

use sbyte_editor::*;
use sbyte_editor::editor::Editor;
use sbyte_editor::commandable::Commandable;


fn result_catcher() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mut editor = SbyteEditor::new();
    match args.get(1) {
        Some(path) => {
            editor.load_file(path)?;
        }
        None => { }
    }

    editor.try_command("setcmd TOGGLE_CONVERTER             EQUALS")?;
    editor.try_command("setcmd CURSOR_DOWN                  J_LOWER")?;
    editor.try_command("setcmd CURSOR_UP                    K_LOWER")?;
    editor.try_command("setcmd CURSOR_LEFT                  H_LOWER")?;
    editor.try_command("setcmd CURSOR_RIGHT                 L_LOWER")?;
    editor.try_command("setcmd CURSOR_LENGTH_DOWN           J_UPPER")?;
    editor.try_command("setcmd CURSOR_LENGTH_UP             K_UPPER")?;
    editor.try_command("setcmd CURSOR_LENGTH_LEFT           H_UPPER")?;
    editor.try_command("setcmd CURSOR_LENGTH_RIGHT          L_UPPER")?;

    editor.try_command("setcmd JUMP_TO_REGISTER             G_UPPER")?;
    editor.try_command("setcmd DELETE                       X_LOWER")?;
    editor.try_command("setcmd YANK                         Y_LOWER")?;
    editor.try_command("setcmd PASTE                        P_LOWER")?;
    editor.try_command("setcmd UNDO                         U_LOWER")?;
    editor.try_command("setcmd REDO                         CTRL+R")?;
    editor.try_command("setcmd CLEAR_REGISTER               ESCAPE,ESCAPE")?;
    editor.try_command("setcmd INCREMENT                    PLUS")?;
    editor.try_command("setcmd DECREMENT                    DASH")?;
    editor.try_command("setcmd BACKSPACE                    BACKSPACE")?;
    editor.try_command("setcmd DELETE                       DELETE")?;

    editor.try_command("setcmd MODE_SET_INSERT              I_LOWER")?;
    editor.try_command("setcmd MODE_SET_INSERT_SPECIAL      I_UPPER")?;
    editor.try_command("setcmd MODE_SET_OVERWRITE           O_LOWER")?;
    editor.try_command("setcmd MODE_SET_OVERWRITE_SPECIAL   O_UPPER")?;
    editor.try_command("setcmd MODE_SET_APPEND              A_LOWER")?;
    editor.try_command("setcmd MODE_SET_SEARCH              SLASH")?;
    editor.try_command("setcmd MODE_SET_CMD                 COLON")?;


    // commands like setcmd run in custom_rc will overwrite whatever was set in the default
    let custom_rc_path = &format!("{}/.sbyterc", env::var("HOME").ok().unwrap());
    if Path::new(custom_rc_path).exists() {
        editor.load_config(custom_rc_path)?;
    }


    editor.main()?;
    Ok(())
}

fn main() {
    match result_catcher() {
            Ok(_) => {}
            Err(error) => {
                println!("Fatal error {}", error);
            }
    }
}
