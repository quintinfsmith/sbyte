use std::cmp::min;
use regex::bytes::Regex;

#[derive(Debug, PartialEq, Eq)]
pub enum ContentError {
    OutOfBounds(usize, usize),
    InvalidDigit(u8, u8)
}

#[derive(Debug, PartialEq, Eq)]
pub enum BitMask {
    And,
    Or,
    Nor,
    Nand,
    Xor
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

    pub fn clear(&mut self) {
        self.content_array.drain(..);
    }

    pub fn get_byte(&self, offset: usize) -> Option<u8> {
        if offset < self.content_array.len() {
            Some(self.content_array[offset])
        } else {
            None
        }
    }
    pub fn set_byte(&mut self, offset: usize, new_byte: u8) -> Result<u8, ContentError> {
        if offset < self.len() {
            let old_byte = self.content_array[offset];
            self.content_array[offset] = new_byte;
            Ok(old_byte)
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

    pub fn increment_byte(&mut self, offset: usize, word_size: usize) -> Result<Vec<u8>, ContentError> {
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
                    if current_byte_offset > offset - (word_size - 1) {
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

    pub fn decrement_byte(&mut self, offset: usize, word_size: usize) -> Result<Vec<u8>, ContentError> {
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
                    if current_byte_offset > offset - (word_size - 1) {
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


    pub fn apply_mask(&mut self, offset: usize, mask: &[u8], operation: BitMask) -> Result<Vec<u8>, ContentError> {
        let mut new_bytes = Vec::new();
        for (i, byte) in mask.iter().enumerate() {
            match self.get_byte(offset + i) {
                Some(v) => {
                    match operation {
                        BitMask::Or => { new_bytes.push(v | *byte); }
                        BitMask::Xor => { new_bytes.push(v ^ *byte); }
                        BitMask::Nor => { new_bytes.push(!(v | *byte)); }
                        BitMask::And => { new_bytes.push(v & *byte); }
                        BitMask::Nand => { new_bytes.push(!(v & *byte)); }
                    }
                }
                None => { }
            }
        }

        let old_bytes = self.remove_bytes(offset, mask.len());

        self.insert_bytes(offset, &new_bytes)?;

        Ok(old_bytes)
    }


    pub fn insert_bytes(&mut self, offset: usize, new_bytes: &[u8]) -> Result<(), ContentError> {
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

    pub fn replace_digit(&mut self, offset: usize, position: u8, digit_value: u8, radix: u8) -> Result<u8, ContentError> {
        match self.get_byte(offset) {
            Some(mut byte) => {
                let mut steps = 256f64.log(radix as f64).ceil() as u8;
                let mut digits = vec![];
                for i in 0 .. steps {
                    if i == position {
                        digits.push(digit_value as u16);
                    } else {
                        digits.push((byte % radix) as u16);
                    }
                    byte /= radix;
                }

                let mut new_byte = 0 as u16;
                for digit in digits.iter().rev() {
                    new_byte *= radix as u16;
                    new_byte += digit;
                }
                if new_byte > u8::MAX as u16 {
                    Err(ContentError::InvalidDigit(digit_value, radix))
                } else {
                    self.set_byte(offset, new_byte as u8)
                }
            }
            None => {
                Err(ContentError::OutOfBounds(offset, self.len()))
            }
        }
    }
}

