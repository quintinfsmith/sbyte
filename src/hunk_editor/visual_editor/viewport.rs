pub struct ViewPort {
    offset: usize,
    width: usize,
    height: usize
}

impl ViewPort {
    pub fn new(width: usize, height: usize) -> ViewPort {
        ViewPort {
            offset: 0,
            width: width,
            height: height
        }
    }
    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_height(&self) -> usize {
        self.height
    }
    pub fn get_offset(&self) -> usize {
        self.offset
    }
    pub fn set_offset(&mut self, new_offset: usize) {
        self.offset = new_offset;
    }
    pub fn set_width(&mut self, new_width: usize) {
        self.width = new_width;
    }
    pub fn set_height(&mut self, new_height: usize) {
        self.height = new_height;
    }
    pub fn set_size(&mut self, new_width: usize, new_height: usize) {
        self.set_width(new_width);
        self.set_height(new_height);
    }
}
