enum PrefixHandlerError {
    DataTooLong
}

trait PrefixHandler {
    fn build_prefix(&mut self, data: Vec<u8>) -> Vec<u8>;
}

struct BigEndianPrefix {
    width: u8
}
impl PrefixHandler for BigEndianPrefix {
    fn build_prefix(&mut self, data: Vec<u8>) -> Result<Vec<u8>, PrefixHandlerError> {
        let max_length = 256.pow(width);
        let mut data_width = data.len();
        if (data_width <= max_length) {
            let mut prefix = Vec::new();
            for i in 0 .. width {
                prefix.push( ((data_width % 16) * 16) + ((data_width / 16) % 16) );
                data_width /= 256;
            }

            Ok(prefix)
        } else {
            Err(PrefixHandlerError::DataTooLong)
        }
    }

    fn decode_prefix(&mut self, prefix: Vec<u8>) -> u64 {
        let mut data_width = 0;
        for n in prefix.iter() {
            data_width *= 256;
            data_width += (n % 16) * 16;
            data_width += n / 16;
        }

        data_width
    }
}

struct LittleEndianPrefix {
    width: u8
}
impl PrefixHandler for LittleEndianPrefix {
    fn build_prefix(&mut self, data: Vec<u8>) -> Result<Vec<u8>, PrefixHandlerError> {
        let max_length = 256.pow(width);
        let mut data_width = data.len();
        if (data_width <= max_length) {
            let mut prefix = Vec::new();
            for i in 0 .. width {
                prefix.insert(0, data_width % 256);
                data_width /= 256;
            }

            Ok(prefix)
        } else {
            Err(PrefixHandlerError::DataTooLong)
        }
    }

    fn decode_prefix(&mut self, prefix: Vec<u8>) -> u64 {
        let mut data_width = 0u64;
        for n in prefix.iter() {
            data_width *= 256;
            data_width += n;
        }

        data_width
    }
}

struct VariableLengthPrefix {}
impl PrefixHandler for VariableLengthPrefix {
}


