use asciibox::RectManager;
use terminal_size::{Width, Height, terminal_size};

pub enum ContentError {

}

struct Content {
    bytes: Vec<u8>
}

impl Content {
    pub fn new(content_bytes: Vec<u8>) -> Content {
        Content {
            bytes: content_bytes
        }
    }

    pub fn insert_bytes(&mut self, new_bytes: Vec<u8>, position: usize) -> Result<(), ContentError> {

    }

    pub fn overwrite_bytes(&mut self, new_bytes: Vec<u8>, position: usize) -> Result<(), ContentError> {

    }

    pub fn remove_chunk(&mut self, position: usize, length: usize) -> Result<(), ContentError> {

    }

    pub fn get_chunk(&self, offset: usize, length: usize) -> Option<Vec<u8>> {

    }

    pub fn get(&self, offset: usize) -> Option<u8> {
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

struct Editor {
}

fn main() {
    println!("Hello, world!");
}
