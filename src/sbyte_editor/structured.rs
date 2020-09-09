pub enum ModificationType {
    INSERT,
    CHANGE,
    REMOVE
}

pub trait StructuredDataHandler {
    fn mod_hook(&self, in_data: Vec<u8>) -> Option<Vec<u8>>;
}

pub struct BigEndianPrefixed {
    prefix_width: usize,
}

impl BigEndianPrefixed {
    pub fn new(width: usize) -> BigEndianPrefixed {
        BigEndianPrefixed { prefix_width: width }
    }

    pub fn decode_prefix(prefix: Vec<u8>) -> u64 {
        let mut data_width: u64 = 0;
        for n in prefix.iter() {
            data_width *= 256;
            data_width += (*n as u64 % 16) * 16;
            data_width += *n as u64 / 16;
        }

        data_width
    }
}

impl StructuredDataHandler for BigEndianPrefixed {
    fn mod_hook(&self, in_data: Vec<u8>) -> Option<Vec<u8>> {
        let mut output;
        let mut tmp_prefix = in_data[0..self.prefix_width].to_vec().clone();
        let expected_data_width = BigEndianPrefixed::decode_prefix(tmp_prefix);
        let real_data_width = (in_data.len() - self.prefix_width) as u64;

        if (expected_data_width == real_data_width) {
            output = None;
        } else {
            let mut new_vec = vec![];
            for i in 0 .. self.prefix_width {
                new_vec.push(((real_data_width / (256u64.pow(i as u32))) % 256u64) as u8)
            }

            new_vec.extend(in_data[self.prefix_width..].iter().copied());

            output = Some(new_vec);
        }

        output
    }
}

pub struct LittleEndianPrefixed {
    prefix_width: usize
}

impl LittleEndianPrefixed {
    pub fn new(width: usize) -> LittleEndianPrefixed {
        LittleEndianPrefixed { prefix_width: width }
    }
    pub fn decode_prefix(prefix: Vec<u8>) -> u64 {
        let mut data_width = 0u64;
        for n in prefix.iter() {
            data_width *= 256;
            data_width += *n as u64;
        }

        data_width
    }
}
impl StructuredDataHandler for LittleEndianPrefixed {
    fn mod_hook(&self, in_data: Vec<u8>) -> Option<Vec<u8>> {
        let mut output;
        let mut tmp_prefix = in_data[0..self.prefix_width].to_vec().clone();
        let expected_data_width = BigEndianPrefixed::decode_prefix(tmp_prefix);
        let real_data_width = (in_data.len() - self.prefix_width) as u64;

        if (expected_data_width == real_data_width) {
            output = None;
        } else {
            let mut new_vec = vec![];
            for i in 0 .. self.prefix_width {
                new_vec.push(((real_data_width / (256u64.pow((self.prefix_width - 1 - i) as u32))) % 256u64) as u8)
            }

            new_vec.extend(in_data[self.prefix_width..].iter().copied());

            output = Some(new_vec);
        }

        output
    }
}

pub struct VariableLengthPrefixed { }
impl VariableLengthPrefixed {
    pub fn build_prefix(number: usize) -> Vec<u8> {
        let mut output = Vec::new();
        let mut working_number = number;
        let mut first_pass = true;
        let mut tmp;
        while working_number > 0 || first_pass {
            tmp = working_number & 0x7F;
            working_number >>= 7;
            if first_pass {
                tmp |= 0x00;
                first_pass = false;
            } else {
                tmp |= 0x80;
            }

            output.push(tmp as u8);
        }
        output.reverse();

        output
    }
}

impl StructuredDataHandler for VariableLengthPrefixed {
    fn mod_hook(&self, in_data: Vec<u8>) -> Option<Vec<u8>> {
        let mut output;
        let mut prefix_width = 0;
        let mut expected_data_width: usize = 0;
        for byte in in_data.iter() {
            expected_data_width <<= 7;
            expected_data_width += (*byte & 0x7F) as usize;
            prefix_width += 1;
            if *byte & 0x80 == 0 {
                break;
            }
        }

        let real_data_width = in_data.len() - prefix_width;

        if (expected_data_width == real_data_width) {
            output = None;
        } else {
            let mut new_vec = VariableLengthPrefixed::build_prefix(real_data_width);

            new_vec.extend(in_data[prefix_width..].iter().copied());

            output = Some(new_vec);
        }

        output
    }
}

