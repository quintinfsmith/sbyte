pub struct Cursor {
    offset: usize,
    length: isize
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor {
            offset: 0,
            length: 1
        }
    }
    pub fn set_length(&mut self, new_length: isize) {
        self.length = new_length;
    }

    pub fn set_offset(&mut self, new_offset: usize) {
        self.offset = new_offset;
    }

    pub fn get_real_offset(&self) -> usize {
        self.offset
    }
    pub fn get_real_length(&self) -> isize {
        self.length
    }

    pub fn get_length(&self) -> usize {
        let output;

        if self.length < 0 {
            output = (0 - self.length) + 1;
        } else {
            output = self.length;
        }

        output as usize
    }

    pub fn get_offset(&self) -> usize {
        let output;

        if self.length < 0 {
            output = ((self.offset as isize) + self.length) as usize;
        } else {
            output = self.offset;
        }

        output
    }
}
