use std::env;

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
            editor.load_config("/etc/sbyte/sbyterc");
        }
        None => {
        }
    };

    editor.main();
}
