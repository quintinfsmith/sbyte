use windows::Win32::System::{Console, Threading};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use std::sync::{Mutex, Arc};
use std::{time, thread};
use std::ops::Deref;


#[inline]
pub fn get_input_reader() -> Reader {
    Reader::new()
}

//DEBUG
use std::fs::OpenOptions;
use std::io::prelude::*;

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

    pub fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), ()> {
        let mut trying = true;
        let mut offset: usize = 0;
        while offset < buffer.len() {
            match self.received_input.try_lock() {
                Ok(ref mut mutex) => {
                    // TODO: This could be improved
                    if mutex.len() > 0 {
                        buffer[offset] = mutex[0];
                        offset += 1;
                        mutex.drain(0..1);
                    }
                }
                Err(e) => { }
            }
        }
        Ok(())
    }

    fn listen(&mut self) {
        let mut receiver = self.received_input.clone();
        let mut kill_signal = self.kill_signal.clone();;
        thread::spawn(move || {
            unsafe {
                match Console::GetStdHandle(Console::STD_INPUT_HANDLE) {
                    Ok(stdinhandle) => {
                        loop {
                            match Threading::WaitForSingleObject(stdinhandle, 50) {
                                WAIT_OBJECT_0 => {
                                    match receiver.try_lock() {
                                        Ok(ref mut mutex) => {
                                            let mut record: [Console::INPUT_RECORD;512] = [Console::INPUT_RECORD::default(); 512];
                                            let mut read = 0;
                                            Console::ReadConsoleInputW(stdinhandle, &mut record, &mut read);
                                            if read > 1 {
                                            }
                                            for i in 0 .. read {
                                                let input_record = record[i as usize];
                                                if input_record.EventType == 1 {
                                                    if input_record.Event.KeyEvent.bKeyDown.as_bool() {
                                                        match input_record.Event.KeyEvent.wVirtualKeyCode {
                                                            0x08 => { // Backspace
                                                                mutex.push(0x7F)
                                                            }
                                                            0x09 => { // Tab
                                                                mutex.push(0x09)
                                                            }
                                                            0x0D => { // Enter
                                                                mutex.push(0x0A)
                                                            }
                                                            0x26 => { // Up Arrow
                                                                mutex.push(0x1B);
                                                                mutex.push(b'[');
                                                                mutex.push(b'A');
                                                            }
                                                            0x25 => { // Left Arrow
                                                                mutex.push(0x1B);
                                                                mutex.push(b'[');
                                                                mutex.push(b'D');
                                                            }
                                                            0x28 => { // Down Arrow
                                                                mutex.push(0x1B);
                                                                mutex.push(b'[');
                                                                mutex.push(b'B');
                                                            }
                                                            0x27 => { // Right Arrow
                                                                mutex.push(0x1B);
                                                                mutex.push(b'[');
                                                                mutex.push(b'C');
                                                            }
                                                            0x2E => { // Del
                                                                mutex.push(0x1B);
                                                                mutex.push(b'[');
                                                                mutex.push(b'3');
                                                                mutex.push(0x7E);
                                                            }
                                                            _ => {
                                                                mutex.push(input_record.Event.KeyEvent.uChar.AsciiChar.0);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(_e) => { }
                                    }
                                }
                                _ => {
                                    match kill_signal.try_lock() {
                                        Ok(mutex) => {
                                            if *mutex.deref() == false {
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
                    Err(_e) => {}
                }
            }
        });
    }
}
