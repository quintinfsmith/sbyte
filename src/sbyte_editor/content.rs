use std::cmp::min;
use regex::bytes::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum ContentError {
    OutOfBounds(usize, usize)
}

pub mod tests;

pub struct Content {
    content_array: Vec<u8>
}

impl Content {
    pub fn new() -> Content {
        Content {
            content_array: Vec::new()
        }
    }

    pub fn get_byte(&self, offset: usize) -> Option<u8> {
        if offset < self.content_array.len() {
            Some(self.content_array[offset])
        } else {
            None
        }
    }
    pub fn set_byte(&mut self, offset: usize, new_byte: u8) -> Result<(), ContentError> {
        if offset < self.len() {
            self.content_array[offset] = new_byte;
            Ok(())
        } else {
            Err(ContentError::OutOfBounds(offset, self.len()))
        }
    }

    pub fn get_chunk(&self, offset: usize, length: usize) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        for i in min(offset, self.len()) .. min(self.len(), offset + length) {
            output.push(self.get_byte(i).unwrap());
        }

        output
    }

    pub fn push(&mut self, byte: u8) {
        self.content_array.push(byte);
    }

    pub fn increment_byte(&mut self, offset: usize) -> Result<Vec<u8>, ContentError> {
        let mut current_byte_offset = offset;
        if self.len() > current_byte_offset {
            let mut current_byte_value = self.content_array[current_byte_offset];
            let mut initial_bytes = vec![];

            loop {
                initial_bytes.insert(0, current_byte_value);
                if current_byte_value < 255 {
                    self.content_array[current_byte_offset] = current_byte_value + 1;
                    break;
                } else {
                    self.content_array[current_byte_offset] = 0;
                    if current_byte_offset > 0 {
                        current_byte_offset -= 1;
                    } else {
                        break;
                    }
                    current_byte_value = self.content_array[current_byte_offset];
                }
            }

            Ok(initial_bytes)
        } else {
            Err(ContentError::OutOfBounds(offset, self.len()))
        }
    }

    pub fn decrement_byte(&mut self, offset: usize) -> Result<Vec<u8>, ContentError> {
        let mut current_byte_offset = offset;

        if self.content_array.len() > current_byte_offset {
            let mut current_byte_value = self.content_array[current_byte_offset];
            let mut initial_bytes = vec![];

            loop {
                initial_bytes.insert(0, current_byte_value);
                if current_byte_value > 0 {
                    self.content_array[current_byte_offset] = current_byte_value - 1;
                    break;
                } else {
                    self.content_array[current_byte_offset] = 255;
                    if current_byte_offset > 0 {
                        current_byte_offset -= 1;
                    } else {
                        break;
                    }
                    current_byte_value = self.content_array[current_byte_offset];
                }
            }

            Ok(initial_bytes)
        } else {
            Err(ContentError::OutOfBounds(offset, self.len()))
        }
    }

    pub fn insert_bytes(&mut self, offset: usize, new_bytes: Vec<u8>) -> Result<(), ContentError> {
        if offset <= self.content_array.len() {
            let mut new_content = self.content_array[0..offset].to_vec();
            let chunk_last = self.content_array[offset..].to_vec();
            new_content.extend(new_bytes.iter().copied());
            new_content.extend(chunk_last.iter().copied());
            self.content_array = new_content;

            Ok(())
        } else {
            Err(ContentError::OutOfBounds(offset, self.len()))
        }

    }

    pub fn remove_bytes(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let output;
        if offset < self.content_array.len() {
            let mut removed_bytes = Vec::new();
            let adj_length = min(self.content_array.len() - offset, length);
            for _ in 0..adj_length {
                removed_bytes.push(self.content_array.remove(offset));
            }
            output = removed_bytes;
        } else {
            output = vec![];
        }

        output
    }

    pub fn as_slice(&self) -> &[u8] {
        self.content_array.as_slice()
    }

    pub fn len(&self) -> usize {
        self.content_array.len()
    }

    // TODO: Overlapping hits
    // eg when look for 33 in 333, there should be 2 hits.
    pub fn find_all(&self, search_for: &str) -> Result<Vec<(usize, usize)>, regex::Error> {
        let working_string = format!("(?-u:{})", search_for);

        match Regex::new(&working_string) {
            Ok(patt) => {
                let mut output = Vec::new();
                for hit in patt.find_iter(self.as_slice()) {
                    output.push((hit.start(), hit.end()))
                }

                output.sort();
                Ok(output)
            }
            Err(e) => {
                Err(e)
            }
        }

    }
}

