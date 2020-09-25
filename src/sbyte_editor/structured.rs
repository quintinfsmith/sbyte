use std::fmt::Display;

pub enum ModificationType {
    INSERT,
    CHANGE,
    REMOVE
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StructureError {
    InvalidInput
}

pub trait StructuredDataHandler {
    //fn mod_request(&self, offset: usize, bytes: Vec<u8>) -> Result<(), StructureError>;
    fn read_in(in_data: Vec<u8>) -> Result<Self, StructureError> where Self: Sized;
    fn as_bytes(&self) -> Vec<u8>;
    fn update(&mut self, new_bytes: Vec<u8>) -> Result<(), StructureError>;

}

pub struct BigEndianPrefixed {
    prefix_width: usize,
    data: Vec<u8>
}

impl BigEndianPrefixed {
    pub fn new(prefix_width: usize, data: Vec<u8>) -> BigEndianPrefixed {
        BigEndianPrefixed {
            prefix_width: prefix_width,
            data: data
        }
    }

    pub fn decode_prefix(prefix: Vec<u8>) -> usize {
        let mut data_width: usize = 0;
        for n in prefix.iter() {
            data_width *= 256;
            data_width += *n as usize;
        }

        data_width
    }

    pub fn build_prefix(&self) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let data_width = self.data.len() as u32;

        let mut _i;
        for i in 0 .. self.prefix_width {
            _i = (self.prefix_width - 1 - i) as u32;
            output.push(((data_width / 256_u32.pow(_i)) % 256_u32.pow(_i + 1)) as u8);
        }

        output
    }
}

impl StructuredDataHandler for BigEndianPrefixed {
    fn read_in(inbytes: Vec<u8>) -> Result<BigEndianPrefixed, StructureError> {
        let mut prefix_width = 0;
        let mut data_width = 0;
        let total_length = inbytes.len();

        while total_length != prefix_width + data_width && prefix_width < total_length {
            prefix_width += 1;
            data_width = BigEndianPrefixed::decode_prefix(inbytes[0..prefix_width].to_vec());
        }

        if total_length == prefix_width {
            Err(StructureError::InvalidInput)
        } else {
            Ok(
                BigEndianPrefixed::new(prefix_width, inbytes[prefix_width..].to_vec())
            )
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let prefix = self.build_prefix();
        output.extend(prefix.iter().copied());
        output.extend(self.data.iter().copied());

        output
    }

    fn update(&mut self, new_bytes: Vec<u8>) -> Result<(), StructureError> {
        match BigEndianPrefixed::read_in(new_bytes) {
            Ok(new_structure) => {
                self.data = new_structure.data;
                self.prefix_width = new_structure.prefix_width;
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }
}

pub struct LittleEndianPrefixed {
    prefix_width: usize,
    data: Vec<u8>
}

impl LittleEndianPrefixed {
    pub fn new(prefix_width: usize, data: Vec<u8>) -> LittleEndianPrefixed {
        LittleEndianPrefixed {
            prefix_width: prefix_width,
            data: data
        }
    }

    pub fn decode_prefix(prefix: Vec<u8>) -> usize {
        let mut data_width = 0usize;
        for n in prefix.iter() {
            data_width *= 256;
            data_width += *n as usize;
        }

        data_width
    }

    pub fn build_prefix(&self) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        let data_width = self.data.len() as u32;

        let mut _i;
        for i in 0 .. self.prefix_width {
            _i = i as u32;
            output.push(((data_width / 256_u32.pow(_i)) % 256_u32.pow(_i + 1)) as u8);
        }

        output
    }
}

impl StructuredDataHandler for LittleEndianPrefixed {
    //fn mod_hook(&self, in_data: Vec<u8>) -> Option<Vec<u8>> {
    //    let mut output;
    //    let mut tmp_prefix = in_data[0..self.prefix_width].to_vec().clone();
    //    let expected_data_width = BigEndianPrefixed::decode_prefix(tmp_prefix);
    //    let real_data_width = (in_data.len() - self.prefix_width) as usize;

    //    if (expected_data_width == real_data_width) {
    //        output = None;
    //    } else {
    //        let mut new_vec = vec![];
    //        for i in 0 .. self.prefix_width {
    //            new_vec.push(((real_data_width / (256usize.pow((self.prefix_width - 1 - i) as u32))) % 256usize) as u8)
    //        }

    //        new_vec.extend(in_data[self.prefix_width..].iter().copied());

    //        output = Some(new_vec);
    //    }

    //    output
    //}
    fn read_in(inbytes: Vec<u8>) -> Result<LittleEndianPrefixed, StructureError> {
        let mut prefix_width = 0;
        let mut data_width = 0;
        let total_length = inbytes.len();

        while total_length != prefix_width + data_width {
            prefix_width += 1;
            data_width = LittleEndianPrefixed::decode_prefix(inbytes[0..prefix_width].to_vec());
        }

        if total_length == prefix_width {
            Err(StructureError::InvalidInput)
        } else {
            Ok(
                LittleEndianPrefixed::new(prefix_width, inbytes[prefix_width..].to_vec())
            )
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut prefix = self.build_prefix();
        output.extend(prefix.iter().copied());
        output.extend(self.data.iter().copied());
        output
    }

    fn update(&mut self, new_bytes: Vec<u8>) -> Result<(), StructureError> {
        match LittleEndianPrefixed::read_in(new_bytes) {
            Ok(new_structure) => {
                self.data = new_structure.data;
                self.prefix_width = new_structure.prefix_width;
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }
}

pub struct VariableLengthPrefixed {
    data: Vec<u8>
}

impl VariableLengthPrefixed {
    pub fn new(data: Vec<u8>) -> VariableLengthPrefixed {
        VariableLengthPrefixed {
            data: data
        }
    }
    pub fn build_prefix(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut working_number = self.data.len();
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
    fn read_in(inbytes: Vec<u8>) -> Result<VariableLengthPrefixed, StructureError> {
        let mut prefix_width = 0;
        let mut expected_data_width: usize = 0;
        for byte in inbytes.iter() {
            expected_data_width <<= 7;
            expected_data_width += (*byte & 0x7F) as usize;
            prefix_width += 1;
            if *byte & 0x80 == 0 {
                break;
            }
        }

        let real_data_width = inbytes.len() - prefix_width;
        let data = inbytes[prefix_width..].to_vec();

        if (real_data_width == data.len()) {
            Ok(VariableLengthPrefixed::new(data))
        } else {
            Err(StructureError::InvalidInput)
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        let mut prefix = self.build_prefix();
        output.extend(prefix.iter().copied());
        output.extend(self.data.iter().copied());
        output
    }

    fn update(&mut self, new_bytes: Vec<u8>) -> Result<(), StructureError> {
        match VariableLengthPrefixed::read_in(new_bytes) {
            Ok(new_structure) => {
                self.data = new_structure.data;
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }
}

