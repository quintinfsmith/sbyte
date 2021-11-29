#[derive(PartialEq, Clone, Copy, Debug, Eq)]
pub enum FormatterRef {
    HEX,
    BIN,
    DEC
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormatterError {
    InvalidDigit(FormatterRef)
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormatterResponse {
    Done,
    Next(usize),
    Failure
}

pub trait Formatter {
    // Return a list of bytes to display,
    //  A map of the same bytes keyed by the input byte-offsets that corresponds thereto
    fn read_in(&self, next_bytes: Vec<u8>) -> (Vec<u8>, HashMap<u8, u8>, FormatterResponse);
    fn ratio(&self) -> Option(u8, u8);

    //fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8>;
    //fn encode_integer(&self, integer: usize) -> Vec<u8>;
}

impl dyn Formatter {
    fn decode_string(&self, string: String) -> Result<Vec<u8>, FormatterError> {
        let bytes = string.as_bytes().to_vec();
        self.decode(bytes)
    }
}

pub struct HexFormatter { }
pub struct HumanFormatter { }
pub struct BinaryFormatter { }
pub struct DecFormatter { }

impl HexFormatter {
    fn hex_char_to_dec_int(&self, hex_char: u8) -> Result<u8, FormatterError> {
        // TODO: Make constant
        let hex_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57,65,66,67,68,69,70];

        match hex_digits.binary_search(&hex_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(_) => {
                Err(FormatterError::InvalidDigit(FormatterRef::HEX))
            }
        }
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
}

impl Formatter for HexFormatter {
    fn read_in(&self, real_bytes: Vec<u8>) -> (Vec<u8>, FormatterResponse) {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        (output_bytes, ConveterResponse::Done)
    }

    fn ratio(&self) -> Option<(u8, u8)> {
        (1, 2)
    }
}

impl BinaryFormatter {
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
}

impl Formatter for BinaryFormatter {
    fn read_in(&self, real_bytes: Vec<u8>) -> (Vec<u8>, FormatterResponse) {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        (output_bytes, FormatterResponse::Done)
    }

    fn ratio(&self) -> Option<(u8,u8)> {
        (1, 8)
    }
}

impl HumanFormatter {
    fn dec_char_to_dec_int(&self, dec_char: u8) -> Result<u8, FormatterError> {
        // TODO: Make constant
        let dec_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57];

        match dec_digits.binary_search(&dec_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(_) => {
                Err(FormatterError::InvalidDigit(FormatterRef::DEC))
            }
        }
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
}

impl Formatter for HumanFormatter {
    fn encode(&self, real_bytes: Vec<u8>) -> Vec<u8> {
        let mut output = Vec::new();
        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output.push(*subbyte);
            }
        }

        output
    }

    fn ratio(&self) -> Option<(u8, u8)> {
        (1, 1)
    }
}

impl DecFormatter {
    fn dec_char_to_dec_int(&self, hex_char: u8) -> Result<u8, FormatterError> {
        // TODO: Make constant
        let dec_digits: Vec<u8> = vec![48,49,50,51,52,53,54,55,56,57];

        match dec_digits.binary_search(&hex_char) {
            Ok(index) => {
                Ok(index as u8)
            }
            Err(_) => {
                Err(FormatterError::InvalidDigit(FormatterRef::DEC))
            }
        }
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

}

impl Formatter for DecFormatter {
    fn read_in(&self, real_bytes: Vec<u8>) -> (Vec<u8>, FormatterResponse) {
        let mut output_bytes: Vec<u8> = Vec::new();

        for byte in real_bytes.iter() {
            for subbyte in self.encode_byte(*byte).iter() {
                output_bytes.push(*subbyte);
            }
        }

        (output_bytes, FormatterResponse::Done)
    }

    fn ratio(&self) -> Option<(u8, u8)> {
        (3, 1)
    }
}
