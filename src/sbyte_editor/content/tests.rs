#[cfg (test)]
mod tests {
    use crate::sbyte_editor::content::{Content, BitMask, ContentError};

    #[test]
    fn test_initialize() {
        let mut content = Content::new();
        assert_eq!(content.as_slice(), []);
    }

    #[test]
    fn test_insert_bytes() {
        let mut content = Content::new();

        assert!(content.insert_bytes(1, &[99]).is_err());
        assert!(content.insert_bytes(0, &[34,35,36,37]).is_ok());
        assert_eq!(content.as_slice(), [34, 35, 36, 37]);

        assert!(content.insert_bytes(2, &[0, 0, 0]).is_ok());
        assert_eq!(content.as_slice(), [34, 35, 0, 0, 0, 36, 37]);
    }

    #[test]
    fn test_push() {
        let mut content = Content::new();
        content.push(24);
        assert_eq!(content.as_slice(), [24]);
    }

    #[test]
    fn test_len() {
        let mut content = Content::new();
        assert_eq!(content.len(), 0);
        content.push(0);
        assert_eq!(content.len(), 1);
    }

    #[test]
    fn test_get_byte() {
        let mut content = Content::new();
        let slice = [45,46,47,23,12];
        content.insert_bytes(0, &slice);
        for (i, byte) in slice.iter().enumerate() {
            assert_eq!(content.get_byte(i), Some(*byte));
        }
    }

    #[test]
    fn test_get_chunk() {
        let mut content = Content::new();
        let slice = [45,46,47,23,12];
        content.insert_bytes(0, &slice);
        assert_eq!(content.get_chunk(0, 5).as_slice(), slice);
        assert_eq!(content.get_chunk(0, 9999).as_slice(), slice);
        assert_eq!(content.get_chunk(0,0).as_slice(), []);
        assert_eq!(content.get_chunk(5, 1).as_slice(), []);
        assert_eq!(content.get_chunk(4, 1).as_slice(), [12]);
    }

    #[test]
    fn test_set_byte() {
        let mut content = Content::new();
        assert!(content.set_byte(0, 0).is_err(), "Failed to throw error when setting byte that is out of bounds");

        content.push(0);
        assert!(content.set_byte(0, 1).is_ok());
    }

    #[test]
    fn test_remove_bytes() {
        let mut content = Content::new();
        content.insert_bytes(0, &[34,35,36,37]);
        content.remove_bytes(2, 1);
        assert_eq!(content.as_slice(), [34, 35, 37]);
        assert_eq!(content.remove_bytes(200, 10).as_slice(), []);
        assert_eq!(content.as_slice(), [34,35,37]);

        assert_eq!(content.remove_bytes(2, 10).as_slice(), [37]);
        assert_eq!(content.as_slice(), [34, 35]);
    }

    #[test]
    fn test_as_slice() {
        let mut content = Content::new();
        assert_eq!(content.as_slice(), []);
        content.push(0);
        assert_eq!(content.as_slice(), [0]);
        content.remove_bytes(0, 1);

        let slice = [0,1,2,3,4,5,6,7,8,9,10];
        content.insert_bytes(0, &slice);

        assert_eq!(content.as_slice(), &slice);
    }

    #[test]
    fn test_find_all() {
        let mut content = Content::new();
        let mut slice = [0x90, 0x91, 0x80, 0x80, 0x90, 0x90, 0x90, 0x90];
        content.insert_bytes(0, &slice);

        assert!(content.find_all("\\x.0").is_err(), "Regex not throwing error when given a bad pattern");
        match content.find_all("\\x80") {
            Ok(hits) => {
                assert_eq!(hits.as_slice(), [(2,3), (3,4)]);
            }
            Err(_) => {}
        }
        match content.find_all("\\x90\\x90") {
            Ok(hits) => {
                assert_eq!(hits.as_slice(), [(4,6), (6,8)]);
            }
            Err(_) => {}
        }

        match content.find_all("\\x00") {
            Ok(empty) => {
                assert_eq!(empty.as_slice(), []);
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_increment_byte() {
        let mut content = Content::new();
        content.push(0);
        content.increment_byte(0, 1);
        assert_eq!(content.get_byte(0), Some(1));
        content.push(255);
        assert_eq!(content.increment_byte(1, 2), Ok(vec![1, 255]));
        assert_eq!(content.get_byte(0), Some(2));
        assert_eq!(content.get_byte(1), Some(0));

        assert!(content.increment_byte(3, 1).is_err());
    }

    #[test]
    fn test_decrement_byte() {
        let mut content = Content::new();
        content.push(255);
        content.decrement_byte(0, 1);
        assert_eq!(content.get_byte(0), Some(254));
        content.push(0);
        assert_eq!(content.decrement_byte(1, 2), Ok(vec![254, 0]));
        assert_eq!(content.get_byte(0), Some(253));
        assert_eq!(content.get_byte(1), Some(255));

        assert!(content.decrement_byte(3, 1).is_err());
    }

    #[test]
    fn test_masks() {
        let mut content = Content::new();
        let or_tests = vec![
            ([0,0], [0xFF, 0xFF], [0xFF, 0xFF]),
            ([0xAA, 0xAA], [0x55, 0x55], [0xFF, 0xFF]),
            ([0xAA, 0xAA], [0x55, 0x00], [0xFF, 0xAA])
        ];

        for (input, mask, output) in or_tests.iter() {
            content.clear();
            content.insert_bytes(0, input);
            content.apply_mask(0, mask, BitMask::Or);
            assert_eq!(content.as_slice(), output);
        }

        let and_tests = vec![
            ([0x6C], [0xFF], [0x6C]),
            ([0x6C], [0x55], [0x44]),
            ([0x00], [0xFF], [0x00]),
            ([0xFF], [0x00], [0x00])
        ];

        for (input, mask, output) in and_tests.iter() {
            content.clear();
            content.insert_bytes(0, input);
            content.apply_mask(0, mask, BitMask::And);
            assert_eq!(content.as_slice(), output);
        }

        let nand_tests = vec![
            ([0x6C], [0xFF], [0x93]),
            ([0x6C], [0x55], [0xBB]),
            ([0x00], [0xFF], [0xFF]),
            ([0xFF], [0x00], [0xFF])
        ];

        for (input, mask, output) in nand_tests.iter() {
            content.clear();
            content.insert_bytes(0, input);
            content.apply_mask(0, mask, BitMask::Nand);
            assert_eq!(content.as_slice(), output);
        }

        let nor_tests = vec![
            ([0x6C], [0xFF], [0x00]),
            ([0x6C], [0x55], [0x82]),
            ([0x00], [0xFF], [0x00]),
            ([0xFF], [0x00], [0x00])
        ];

        for (input, mask, output) in nor_tests.iter() {
            content.clear();
            content.insert_bytes(0, input);
            content.apply_mask(0, mask, BitMask::Nor);
            assert_eq!(content.as_slice(), output);
        }

        let xor_tests = vec![
            ([0x6C], [0xFF], [0x93]),
            ([0x6C], [0x55], [0x39]),
            ([0x00], [0xFF], [0xFF]),
            ([0xFF], [0xFF], [0x00])
        ];

        for (input, mask, output) in xor_tests.iter() {
            content.clear();
            content.insert_bytes(0, input);
            content.apply_mask(0, mask, BitMask::Xor);
            assert_eq!(content.as_slice(), output);
        }
    }

    #[test]
    fn test_replace_digit() {
        let mut content = Content::new();
        content.insert_bytes(0, &[0x00]);


        // Test base 16
        content.replace_digit(0, 0, 1, 16);
        assert_eq!(content.as_slice(), &[0x01]);
        content.replace_digit(0, 1, 15, 16);
        assert_eq!(content.as_slice(), &[0xF1]);
        assert!(content.replace_digit(0, 0, 16, 16).is_err());

        content.set_byte(0,0);

        // Test base 2
        content.replace_digit(0, 0, 1, 2);
        assert_eq!(content.as_slice(), &[0b00000001]);
        content.replace_digit(0, 1, 1, 2);
        assert_eq!(content.as_slice(), &[0b00000011]);
        content.replace_digit(0, 2, 1, 2);
        assert_eq!(content.as_slice(), &[0b00000111]);
        content.replace_digit(0, 3, 1, 2);
        assert_eq!(content.as_slice(), &[0b00001111]);
        content.replace_digit(0, 4, 1, 2);
        assert_eq!(content.as_slice(), &[0b00011111]);
        content.replace_digit(0, 5, 1, 2);
        assert_eq!(content.as_slice(), &[0b00111111]);
        content.replace_digit(0, 6, 1, 2);
        assert_eq!(content.as_slice(), &[0b01111111]);
        content.replace_digit(0, 7, 1, 2);
        assert_eq!(content.as_slice(), &[0b11111111]);
        assert!(content.replace_digit(0, 0, 2, 2).is_err());

        // Test base 10
        content.set_byte(0,0x00);
        content.replace_digit(0, 0, 1, 10);
        assert_eq!(content.as_slice(), &[1]);
        content.replace_digit(0, 1, 9, 10);
        assert_eq!(content.as_slice(), &[91]);
        content.replace_digit(0, 2, 1, 10);
        assert_eq!(content.as_slice(), &[191]);

        assert!(content.replace_digit(0,2,2,10).is_err(), "Should not be able to convert to number > 255");

        content.replace_digit(0, 1, 2, 10);
        assert_eq!(content.as_slice(), &[121]);
        assert!(content.replace_digit(0, 2,2,10).is_ok());
        assert_eq!(content.as_slice(), &[221]);

        assert_eq!(content.replace_digit(10, 0, 2, 3), Err(ContentError::OutOfBounds(10, 1)));

    }
}
