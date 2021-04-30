
#[derive(PartialEq, Clone, Copy, Debug, Eq)]
pub enum ConverterRef {
    HEX,
    BIN,
    DEC
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConverterError {
    InvalidDigit(ConverterRef)
}

pub trait Converter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8>;
    fn encode_byte(&self, byte: u8) -> Vec<u8>;

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError>;
    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError>;
    fn encode_integer(&self, integer: usize) -> Vec<u8>;

    fn radix(&self) -> u32;
}
impl dyn Converter {
    fn decode_string(&self, string: String) -> Result<Vec<u8>, ConverterError> {
        let bytes = string.as_bytes().to_vec();
        self.decode(bytes)
    }
}

pub struct HexConverter { }
pub struct HumanConverter { }
pub struct BinaryConverter { }
pub struct DecConverter { }

impl HexConverter {
    fn hex_char_to_dec_int(&self, hex_char: u8) -> Result<u8, ConverterError> {
        // TODO: Make constant
        let hex_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];

        match hex_digits.binary_search(&hex_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(_) => {
                Err(ConverterError::InvalidDigit(ConverterRef::HEX))
            }
        }
    }
}

impl Converter for HexConverter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        output_bytes
    }

    fn encode_byte(&self, byte: u8) -> Vec<u8> {
        let hex_digits = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];

        let mut output = Vec::new();

        output.push(hex_digits[(byte / 16) as usize]);
        output.push(hex_digits[(byte % 16) as usize]);

        output
    }

    fn encode_integer(&self, mut integer: usize) -> Vec<u8> {
        let hex_digits = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];
        let mut output = Vec::new();
        let mut tmp_hex_digit;
        let passes = (integer as f64).log(16.0).ceil() as usize;
        for _ in 0 .. passes {
            tmp_hex_digit = integer % 16;
            output.insert(0, hex_digits[tmp_hex_digit]);
            integer /= 16;
        }

        output
    }

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError> {
        let mut output_bytes: Vec<u8> = Vec::new();

        let mut byte_value: u8;
        let mut lode_byte = 0;
        for (i, byte) in bytes.iter().rev().enumerate() {
            match self.hex_char_to_dec_int(*byte) {
                Ok(decimal) => {
                    byte_value = decimal;
                    lode_byte += byte_value * ((16_u32.pow((i % 2) as u32)) as u8);

                    if i % 2 != 0 {
                        output_bytes.push(lode_byte);
                        lode_byte = 0;
                    }
                }
                Err(e) => {
                    Err(e)?;
                }
            }
        }

        if lode_byte != 0 {
            output_bytes.push(lode_byte);
        }

        output_bytes.reverse();

        Ok(output_bytes)
    }

    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError> {
        let mut output_number: usize = 0;

        for byte in byte_string.iter() {
            match self.hex_char_to_dec_int(*byte) {
                Ok(decimal_int) => {
                    output_number *= 16;
                    output_number += decimal_int as usize;
                }
                Err(e) => {
                    Err(e)?;
                }
            }
        }

        Ok(output_number)
    }
    fn radix(&self) -> u32 {
        16
    }
}

impl Converter for BinaryConverter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        output_bytes
    }

    fn encode_byte(&self, byte: u8) -> Vec<u8> {
        let mut output = Vec::new();
        for i in 0 .. 8 {
            if byte & (1 << i) == 0 {
                output.insert(0, 48); // 0
            } else {
                output.insert(0, 49); // 1
            }
        }

        output
    }

    fn encode_integer(&self, mut integer: usize) -> Vec<u8> {
        let bits = vec![48,49];
        let mut output = Vec::new();
        let mut tmp_bin_digit;

        let passes = (integer as f64).log(2.0).ceil() as usize;
        for _ in 0 .. passes {
            tmp_bin_digit = integer % 2;
            output.insert(0, bits[tmp_bin_digit]);
            integer /= 2;
        }

        output
    }

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError> {
        let mut output_bytes: Vec<u8> = Vec::new();

        let mut lode_byte = 0;


        for (i, byte) in bytes.iter().enumerate() {
            lode_byte *= 2;
            if *byte == 48 || *byte == 49 {
                lode_byte += *byte - 48;
            } else {
                Err(ConverterError::InvalidDigit(ConverterRef::BIN))?;
            }

            if i == 7 || i == bytes.len() - 1 {
                output_bytes.push(lode_byte);
                lode_byte = 0;
            }
        }

        Ok(output_bytes)
    }

    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError> {
        let mut output_number: usize = 0;

        for byte in byte_string.iter() {
            output_number *= 2;
            if *byte == 48 || *byte == 49 {
                output_number += (*byte as usize) - 48;
            } else {
                Err(ConverterError::InvalidDigit(ConverterRef::BIN))?;
            }

        }

        Ok(output_number)
    }

    fn radix(&self) -> u32 {
        2
    }
}

impl HumanConverter {
    fn dec_char_to_dec_int(&self, dec_char: u8) -> Result<u8, ConverterError> {
        // TODO: Make constant
        let dec_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57];

        match dec_digits.binary_search(&dec_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(_) => {
                Err(ConverterError::InvalidDigit(ConverterRef::DEC))
            }
        }
    }
}

impl Converter for HumanConverter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output = Vec::new();
        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output.push(*subbyte);
            }
        }

        output
    }

    fn encode_byte(&self, byte: u8) -> Vec<u8> {
        let mut output = Vec::new();
        match byte {
            10 => {
                output.push(226);
                output.push(134);
                output.push(178);
            }
            0..=31 => {
                output.push(46);
            }
            127..=255 => {
                output.push(46);
            }
            _ => {
                output.push(byte);
            }
        }

        output
    }

    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError> {
        Ok(bytes)
    }

    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError> {
        let mut output_number: usize = 0;

        for byte in byte_string.iter() {
            match self.dec_char_to_dec_int(*byte) {
                Ok(decimal_int) => {
                    output_number *= 10;
                    output_number += decimal_int as usize;
                }
                Err(e) => {
                    Err(e)?;
                }
            }
        }
        Ok(output_number)
    }

    fn encode_integer(&self, mut integer: usize) -> Vec<u8> {
        let digits = vec![48,49,50,51,52,53,54,55,56,57];
        let mut did_first_pass = false;
        let mut output = Vec::new();
        let mut test_byte;
        while integer > 0 || ! did_first_pass {
            test_byte = integer % 10;
            output.push(digits[test_byte]);
            integer /= 10;
            did_first_pass = true;
        }

        output
    }

    fn radix(&self) -> u32 {
        256
    }
}

impl DecConverter {
    fn dec_char_to_dec_int(&self, hex_char: u8) -> Result<u8, ConverterError> {
        // TODO: Make constant
        let dec_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57];

        match dec_digits.binary_search(&hex_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(_) => {
                Err(ConverterError::InvalidDigit(ConverterRef::DEC))
            }
        }
    }
}

impl Converter for DecConverter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        output_bytes
    }

    fn encode_byte(&self, byte: u8) -> Vec<u8> {
        let dec_digits = vec![48,49,50,51,52,53,54,55,56,57];
        let mut output = Vec::new();

        output.push(dec_digits[((byte / 100) % 10) as usize]);
        output.push(dec_digits[((byte / 10) % 10) as usize]);
        output.push(dec_digits[(byte % 10) as usize]);

        output
    }

    fn encode_integer(&self, mut integer: usize) -> Vec<u8> {
        let dec_digits = vec![48,49,50,51,52,53,54,55,56,57];
        let mut output = Vec::new();
        let mut tmp_dec_digit;
        let passes = (integer as f64).log(10.0).ceil() as usize;
        for _ in 0 .. passes {
            tmp_dec_digit = integer % 10;
            output.insert(0, dec_digits[tmp_dec_digit]);
            integer /= 10;
        }

        output
    }

    // convert vector of ascii decimal digits (eg "123456" -> vec![1, 226, 64])
    fn decode(&self, bytes: Vec<u8>) -> Result<Vec<u8>, ConverterError> {
        let mut tmpint: usize = 0;
        for byte in bytes.iter() {
            match self.dec_char_to_dec_int(*byte) {
                Ok(decimal) => {
                    tmpint *= 10;
                    tmpint += decimal as usize;
                }
                Err(e) => {
                    Err(e)?;
                }
            }
        }

        let mut output_bytes: Vec<u8> = Vec::new();
        let mut first_pass = true;
        while first_pass || tmpint > 0 {
            output_bytes.push((tmpint % 256) as u8);
            tmpint /= 256;
            first_pass = false;
        }

        output_bytes.reverse();

        Ok(output_bytes)
    }

    fn decode_integer(&self, byte_string: Vec<u8>) -> Result<usize, ConverterError> {
        let mut some_number = 0;
        for byte in byte_string.iter() {
            match self.dec_char_to_dec_int(*byte) {
                Ok(decimal_int) => {
                    some_number *= 10;
                    some_number += decimal_int as usize;
                }
                Err(e) => {
                    Err(e)?;
                }
            }
        }

        Ok(some_number)
    }

    fn radix(&self) -> u32 {
        10
    }
}
