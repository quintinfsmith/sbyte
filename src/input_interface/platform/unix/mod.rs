use std::io::{Read, stdin};
pub type Reader = std::io::Stdin;

#[inline]
pub fn get_input_reader() -> Reader {
    stdin()
}

