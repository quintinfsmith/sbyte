use std::env;
use std::path::Path;
use std::error::Error;
use dirs::home_dir;

pub mod editor;
pub mod input_interface;
pub mod console_displayer;
pub mod shell;

use input_interface::InputInterface;
use console_displayer::FrontEnd;
use shell::Shell;

fn result_catcher() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let mut shell = Shell::new();
    match args.get(1) {
        Some(path) => {
            shell.get_editor_mut().load_file(path)?;
        }
        None => { }
    }
    let frontend = FrontEnd::new();
    let mut input_interface = InputInterface::new(shell, frontend);

    // commands like setcmd run in custom_rc will overwrite whatever was set in the default
    match home_dir() {
        Some(home) => {
            let custom_rc_path = &format!("{:?}/.sbyterc", home);
            if Path::new(custom_rc_path).exists() {
                input_interface.load_config(custom_rc_path)?;
            }
        }
        None => {}
    }

    input_interface.main()?;

    Ok(())
}


fn main() {
    match result_catcher() {
        Ok(_) => {}
        Err(error) => {
            eprintln!("Fatal error {}", error);
        }
    }
}
