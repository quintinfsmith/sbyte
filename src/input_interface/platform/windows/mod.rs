use windows::Win32::System::{Console, Threading};

use io
use std::sync::{Mutex, Arc};

pub struct Reader {
    received_input: Arc<Mutex<Vec<u8>>>,
    kill_signal: Arc<Mutex<bool>>
}

impl Reader {
    pub fn new() -> Reader {
        let mut output = Reader {
            received_input: Arc::new(Mutex::new(Vec::new())),
            kill_signal: Arc::new(Mutex::new(false))
        };

        output.listen();

        output
    }

    pub fn read_exact(&mut self, buffer: &mut [u8]) {
        let mut trying = true;
        let mut offset: usize = 0;
        while offset < buffer.len() {
            match self.received_input.try_lock() {
                Ok(ref mut mutex) => {
                    // TODO: This could be improved
                    if mutex.len() > 0 {
                        buffer[offset] = mutex[0];
                        mutex.drain(0);
                    }
                }
                Err(e) = { }
            }
        }
    }

    fn listen(&mut self) {
        let mut receiver = self.received_input.clone();
        let mut kill_signal = self.kill_signal.clone();;
        thread::spawn(move || {
            match Console::GetStdHandle(Console::STD_INPUT_HANDLE) {
                Ok(stdinhandle) => {
                    loop {
                        match Threading::WaitForSingleObject(stdinhandle, 50) {
                            WAIT_OBJECT_0 => {
                                match receiver.try_lock() {
                                    Ok(ref mut mutex) => {
                                        let mut record: [Console::INPUT_RECORD;512] = [0;512];
                                        let mut read = 0;
                                        Console::ReadConsoleInputW(stdinhandle, record, &mut read);
                                    }
                                    Err(_e) => { }
                                }
                            }
                            _ => {
                                match kill_signal.try_lock() {
                                    Ok(ref mut mutex) => {
                                        if mutex {
                                            break;
                                        }
                                    }
                                    Err(_e) => {
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
