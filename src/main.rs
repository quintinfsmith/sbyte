use std::env;

pub mod hunk_editor;

use hunk_editor::*;
use hunk_editor::editor::Editor;
use hunk_editor::commandable::Commandable;
use hunk_editor::commandable::inputter::function_ref::FunctionRef;


fn main() {
    let args: Vec<String> = env::args().collect();
    let mut editor = HunkEditor::new();
    editor.assign_line_command("q".to_string(), FunctionRef::KILL);
    editor.assign_line_command("w".to_string(), FunctionRef::SAVE);
    editor.assign_line_command("wq".to_string(), FunctionRef::SAVEKILL);
    editor.assign_line_command("find".to_string(), FunctionRef::JUMP_TO_NEXT);
    editor.assign_line_command("insert".to_string(), FunctionRef::INSERT);
    editor.assign_line_command("overwrite".to_string(), FunctionRef::OVERWRITE);
    match args.get(1) {
        Some(path) => {
            editor.load_file(path.to_string());
        }
        None => {
        }
    };
    editor.main();
}
