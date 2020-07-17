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
    editor.load_file(args.get(1).unwrap().to_string());
    editor.main();
}
