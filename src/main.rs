use std::env;
use std::path::Path;

pub mod sbyte_editor;

use sbyte_editor::*;
use sbyte_editor::editor::Editor;
use sbyte_editor::commandable::Commandable;


fn main() {
    let args: Vec<String> = env::args().collect();
    let mut editor = SbyteEditor::new();

    match args.get(1) {
        Some(path) => {
            editor.load_file(path);
        }
        None => {
        }
    };

    let dev_rc_path = "sbyterc";
    let default_rc_path = "/etc/sbyte/sbyterc";
    if Path::new(dev_rc_path).exists() {
        editor.load_config(dev_rc_path);
    } else if Path::new(default_rc_path).exists() {
        editor.load_config(default_rc_path);
    }

    // commands like setcmd run in custom_rc will overwrite whatever was set in the default
    let custom_rc_path = &format!("{}/.sbyterc", env::var("HOME").ok().unwrap());
    if Path::new(custom_rc_path).exists() {
        editor.load_config(custom_rc_path);
    }



    editor.main();
}
