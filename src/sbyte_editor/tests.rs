#[cfg (test)]
mod tests {
    use crate::sbyte_editor::{BackEnd, ConverterRef, HexConverter, BinaryConverter, DecConverter, SbyteError, parse_words, string_to_integer, string_to_bytes};
    use std::{time, thread};

    #[test]
    fn test_initializes_empty() {
        let editor = BackEnd::new();
        assert_eq!(editor.active_content.as_slice(), []);
    }

    #[test]
    fn test_load_file() {
        let mut editor = BackEnd::new();
        editor.load_file("src/testfiles/00").expect("Couldn't open file");
        assert_eq!(editor.active_content.as_slice(), "TESTFILECONTENTS".as_bytes());
        assert_eq!(editor.get_chunk(0, 4).as_slice(), "TEST".as_bytes());
        assert_eq!(editor.get_chunk(0, 44).as_slice(), "TESTFILECONTENTS".as_bytes());
    }


    #[test]
    fn test_insert_bytes() {
        let mut editor = BackEnd::new();

        editor.insert_bytes(0, &[65]);
        assert_eq!(editor.active_content.as_slice(), [65]);

        // inserting out of range should ignore insertion
        editor.insert_bytes(10, &[65]);
        assert_eq!(editor.active_content.as_slice(), [65]);

        editor.set_cursor_offset(1);
        editor.insert_bytes_at_cursor(&[13,15,16]);
        assert_eq!(editor.active_content.as_slice(), [65, 13,15,16]);
    }

    #[test]
    fn test_overwrite_bytes() {
        let mut editor = BackEnd::new();

        editor.overwrite_bytes(0, &[65]);
        assert_eq!(editor.active_content.as_slice(), [65]);

        editor.overwrite_bytes(0, &[24, 25, 26]);
        assert_eq!(editor.active_content.as_slice(), [24, 25, 26]);

        // overwriting out of range should ignore overwrite
        editor.overwrite_bytes(10, &[65]);
        assert_eq!(editor.active_content.as_slice(), [24, 25, 26]);

        editor.set_cursor_offset(1);
        editor.overwrite_bytes_at_cursor(&[13,15,16]);
        assert_eq!(editor.active_content.as_slice(), [24, 13,15,16]);
    }

    #[test]
    fn test_remove_bytes() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, &[65]);

        assert_eq!(editor.remove_bytes(0, 1), &[65]);
        assert_eq!(editor.active_content.as_slice(), []);
        assert_eq!(editor.remove_bytes(1000, 300), &[]);

    }
    #[test]
    fn test_remove_bytes_at_cursor() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, &[65]);
        editor.set_cursor_offset(0);
        assert_eq!(editor.remove_bytes_at_cursor(), &[65]);
        assert_eq!(editor.active_content.as_slice(), []);
    }

    #[test]
    fn test_yanking() {
        let mut editor = BackEnd::new();

        editor.insert_bytes(0, &[65, 66, 67, 68]);

        editor.make_selection(1, 3);
        assert_eq!(editor.get_selected().as_slice(), [66, 67, 68]);

        editor.copy_selection();
        assert_eq!(editor.get_clipboard().as_slice(), [66, 67, 68]);
    }

    #[test]
    fn test_find() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, &[65, 66, 0, 0, 65, 65, 66, 60, 0x30]);
        assert_eq!(
            editor.find_all("AB").ok(),
            Some(vec![(0,2), (5,7)])
        );
        assert_eq!(editor.find_after("AB", 2).ok().unwrap(), Some((5, 7)));

        assert_eq!(
            editor.find_all("\\x4.").ok(),
            Some(vec![(0,1), (1,2), (4,5), (5,6), (6,7)])
        );
        assert_eq!(
            editor.find_all("\\x.0").ok(),
            Some(vec![(2,3), (3,4), (8,9)])
        );

        editor.remove_bytes(0, 9);

        editor.insert_bytes(
            0,
            &[0xFF, 0x0F, 0xF0, 0x77, 0x7F, 0xF7, 0xC0, 0x0C]
        );

        assert_eq!(
            editor.find_all("\\b.1111111").ok(),
            Some(vec![(0,1), (4,5)])
        );

        assert_eq!(
            editor.find_all("\\b.111.111").ok(),
            Some(vec![(0,1), (3,4), (4,5), (5,6)])
        );

        assert_eq!(
            editor.find_all("\\b0000....").ok(),
            Some(vec![(1,2), (7,8)])
        );

        assert!(editor.find_all("\\x..").is_err());
        assert!(editor.find_all("\\b0000000b").is_err());
        assert!(editor.find_all("\\b00000.0b").is_err());
    }

    #[test]
    fn test_increment_byte() {
        let mut editor = BackEnd::new();
        assert!(editor.increment_byte(0, 1).is_err(), "Didn't throw error when incrementing outside of valid range.");

        editor.insert_bytes(0, &[0]);
        assert!(editor.increment_byte(0, 1).is_ok(), "Failed to increment byte");

    }

    #[test]
    fn test_decrement_byte() {
        let mut editor = BackEnd::new();
        assert!(editor.decrement_byte(0, 1).is_err(), "Didn't throw error when decrementing outside of valid range.");

        editor.insert_bytes(0, &[1]);
        assert!(editor.decrement_byte(0, 1).is_ok(), "Failed to decrement byte");

        let task = editor.undo_stack.last();
        assert!(task.is_some());
        assert_eq!(task.unwrap().0, 0);
        assert_eq!(task.unwrap().1, 1);
        assert_eq!(task.unwrap().2, &[1]);
    }

    #[test]
    fn test_set_user_msg() {
        let mut editor = BackEnd::new();
        editor.set_user_msg("Test MSG");
        assert_eq!(editor.get_user_msg(), Some(&"Test MSG".to_string()));
    }
    #[test]
    fn test_set_user_error_msg() {
        let mut editor = BackEnd::new();
        editor.set_user_error_msg("Test MSG");
        assert_eq!(editor.get_user_error_msg(), Some(&"Test MSG".to_string()));
    }

    #[test]
    fn test_undo() {
        let mut editor = BackEnd::new();
        assert!(editor.undo().is_err(), "Didn't raise error when trying to undo from an empty stack");

        editor.insert_bytes(0, &[0]);
        assert!(editor.undo().is_ok(), "Failed to undo");

        editor.insert_bytes(0, &[0]);
        editor.insert_bytes(0, &[0]);
        editor.insert_bytes(0, &[0]);
        assert_eq!(editor.undo_stack.len(), 3);
        assert!(editor.undo().is_ok());
        assert_eq!(editor.undo_stack.len(), 0, "Sequential tasks aren't getting undone");

        editor.insert_bytes(0, &[0]);
        thread::sleep(time::Duration::from_nanos(100_000_000));
        editor.insert_bytes(0, &[0]);
        assert!(editor.undo().is_ok());
        assert_eq!(editor.undo_stack.len(), 1, "Undo is merging unrelated tasks");


    }

    #[test]
    fn test_redo() {
        let mut editor = BackEnd::new();

        editor.insert_bytes(0, &[0]);
        assert!(editor.redo().is_err(), "Didn't raise error when trying to redo from an (presumably) empty stack");
        editor.undo();

        assert!(editor.redo().is_ok(), "Failed to redo");

        assert_eq!(editor.redo_stack.len(), 0);

        thread::sleep(time::Duration::from_nanos(60_000_000));

        editor.insert_bytes(0, &[0]);
        editor.insert_bytes(0, &[0]);
        editor.undo();
        assert_eq!(editor.redo_stack.len(), 2);
        editor.undo();
        assert_eq!(editor.redo_stack.len(), 3);
        editor.redo();
        assert_eq!(editor.redo_stack.len(), 1);
        editor.redo();
        assert_eq!(editor.redo_stack.len(), 0);

    }

    #[test]
    fn test_set_active_converter() {
        let mut editor = BackEnd::new();
        editor.set_active_converter(ConverterRef::HEX);
        assert_eq!(editor.get_active_converter_ref(), ConverterRef::HEX);
        editor.set_active_converter(ConverterRef::BIN);
        assert_eq!(editor.get_active_converter_ref(), ConverterRef::BIN);
        editor.set_active_converter(ConverterRef::DEC);
        assert_eq!(editor.get_active_converter_ref(), ConverterRef::DEC);
    }

    #[test]
    fn test_viewport_size() {
        let mut editor = BackEnd::new();
        editor.set_viewport_size(20,20);
        assert_eq!(editor.get_viewport_size(), (20, 20));
    }

    #[test]
    fn test_viewport_offset() {
        let mut editor = BackEnd::new();
        editor.set_viewport_offset(10);
        assert_eq!(editor.get_viewport_offset(), 10);
    }

    #[test]
    fn test_active_file_path() {
        let mut editor = BackEnd::new();
        assert_eq!(editor.get_active_file_path(), None);

        editor.active_file_path = Some("testpath".to_string());
        assert_eq!(editor.get_active_file_path(), Some(&"testpath".to_string()));
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = BackEnd::new();
        editor.set_viewport_size(3,10);
        editor.insert_bytes(0, &[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]);
        editor.set_cursor_offset(1);
        assert_eq!(editor.get_cursor_offset(), 1);
        editor.cursor_next_byte();
        assert_eq!(editor.get_cursor_offset(), 2);
        editor.cursor_prev_byte();
        assert_eq!(editor.get_cursor_offset(), 1);

        editor.set_cursor_offset(3);
        editor.cursor_next_line();
        assert_eq!(editor.get_cursor_offset(), 6);
        editor.cursor_prev_line();
        assert_eq!(editor.get_cursor_offset(), 3);

        editor.set_cursor_offset(9);
        editor.set_cursor_length(1);
        editor.cursor_decrease_length();
        assert_eq!(editor.get_cursor_offset(), 8);
        assert_eq!(editor.get_cursor_length(), 2);
        editor.cursor_decrease_length();
        assert_eq!(editor.get_cursor_offset(), 7);
        assert_eq!(editor.get_cursor_length(), 3);
        editor.cursor_increase_length();
        assert_eq!(editor.get_cursor_offset(), 8);
        assert_eq!(editor.get_cursor_length(), 2);

        editor.set_cursor_offset(9);
        editor.set_cursor_length(1);
        editor.cursor_increase_length();
        assert_eq!(editor.get_cursor_offset(), 9);
        assert_eq!(editor.get_cursor_length(), 2);
        editor.cursor_increase_length();
        assert_eq!(editor.get_cursor_offset(), 9);
        assert_eq!(editor.get_cursor_length(), 3);
        editor.cursor_decrease_length();
        assert_eq!(editor.get_cursor_offset(), 9);
        assert_eq!(editor.get_cursor_length(), 2);

        editor.set_cursor_offset(18);
        editor.set_cursor_length(4);
        assert_eq!(editor.get_cursor_length(), 1, "Cursor length was set to overflow");

        editor.set_cursor_offset(9);
        editor.set_cursor_length(1);
        editor.cursor_increase_length_by_line();
        assert_eq!(editor.get_cursor_length(), 4);
        editor.cursor_increase_length_by_line();
        assert_eq!(editor.get_cursor_length(), 7);
        editor.cursor_decrease_length_by_line();
        assert_eq!(editor.get_cursor_length(), 4);
        editor.cursor_decrease_length_by_line();
        assert_eq!(editor.get_cursor_length(), 1);
        editor.cursor_decrease_length_by_line();
        assert_eq!(editor.get_cursor_length(), 4);
        assert_eq!(editor.get_cursor_offset(), 6);
        editor.cursor_decrease_length_by_line();
        assert_eq!(editor.get_cursor_length(), 7);
        assert_eq!(editor.get_cursor_offset(), 3);

        editor.cursor_increase_length_by_line();
        editor.cursor_increase_length_by_line();
        assert_eq!(editor.get_cursor_length(), 1);
        assert_eq!(editor.get_cursor_offset(), 9);
    }

    #[test]
    fn test_get_display_ratio() {
        let mut editor = BackEnd::new();
        editor.set_active_converter(ConverterRef::HEX);
        assert_eq!(editor.get_display_ratio(), 3);

        editor.set_active_converter(ConverterRef::BIN);
        assert_eq!(editor.get_display_ratio(), 9);

        editor.set_active_converter(ConverterRef::DEC);
        assert_eq!(editor.get_display_ratio(), 4);
    }

    #[test]
    fn test_user_feedback() {
        let mut editor = BackEnd::new();
        let test_msg = "Test MSG";
        let test_error = "Test Error MSG";
        assert!(editor.get_user_msg().is_none());
        editor.set_user_msg(test_msg);
        assert_eq!(editor.get_user_msg(), Some(&test_msg.to_string()));
        editor.unset_user_msg();
        assert!(editor.get_user_msg().is_none());

        assert!(editor.get_user_error_msg().is_none());
        editor.set_user_error_msg(test_error);
        assert_eq!(editor.get_user_error_msg(), Some(&test_error.to_string()));
        editor.unset_user_error_msg();
        assert!(editor.get_user_error_msg().is_none());
    }

    // TODO: Move these tests
    //#[test]
    //fn test_try_command() {
    //    let mut editor = BackEnd::new();
    //    let mut result = editor.try_command("badcmd");
    //    match result {
    //        Ok(_) => {
    //            assert!(false);
    //        }
    //        Err(e) => {
    //            assert_eq!(e, SbyteError::InvalidCommand("badcmd".to_string()));
    //        }
    //    }

    //    editor.assign_line_command("testcmd", "TEST");
    //    result = editor.try_command("testcmd arg1 arg2");
    //    assert_eq!(result, Ok(("TEST".to_string(), vec!["arg1".to_string(), "arg2".to_string()])));

    //    result = editor.try_command("");
    //    assert_eq!(result, Err(SbyteError::NoCommandGiven));
    //}

    #[test]
    fn test_replace() {
        let mut editor = BackEnd::new();
        let slice = [0,4,2,0];
        editor.insert_bytes(0, &slice);

        editor.replace("\\x00", &[87]);
        assert_eq!(editor.get_active_content(), &[87,4,2,87]);
        editor.replace("\\x04", &[55,66,77]);
        assert_eq!(editor.get_active_content(), &[87,55,66,77,2,87]);

    }

    #[test]
    fn test_masking() {
        let mut editor = BackEnd::new();
        // Check individual functions once, more in depth tests are in content/tests.rs
        editor.insert_bytes(0, &[0x56, 0x56, 0x56, 0x56, 0x56]);
        editor.apply_or_mask(&[0x65]); //0x77
        editor.set_cursor_offset(1);
        editor.apply_nor_mask(&[0x65]); //0x88
        editor.set_cursor_offset(2);
        editor.apply_xor_mask(&[0x65]); //0x33
        editor.set_cursor_offset(3);
        editor.apply_and_mask(&[0x65]); //0x44
        editor.set_cursor_offset(4);
        editor.apply_nand_mask(&[0x65]); //0xBB
        assert_eq!(editor.get_active_content(), &[0x77, 0x88, 0x33, 0x44, 0xBB]);

        // Check that the mask gets repeated to the length of the cursor
        editor.set_cursor_offset(0);
        editor.set_cursor_length(3);
        editor.apply_or_mask(&[0xFF]);
        assert_eq!(editor.get_active_content(), &[0xFF, 0xFF, 0xFF, 0x44, 0xBB]);

        // Check that the mask gets clipped to the length of the cursor
        editor.set_cursor_offset(0);
        editor.set_cursor_length(2);
        editor.apply_and_mask(&[0x00, 0x00, 0x00]);
        assert_eq!(editor.get_active_content(), &[0x00, 0x00, 0xFF, 0x44, 0xBB]);

        // Check that the mask does NOT get clipped to the length of the cursor if the cursor is length 1
        editor.set_cursor_offset(0);
        editor.set_cursor_length(1);
        editor.apply_nor_mask(&[0x00, 0x00]);
        assert_eq!(editor.get_active_content(), &[0xFF, 0xFF, 0xFF, 0x44, 0xBB]);
    }

    #[test]
    fn test_bitwise_not() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, &[0xAA, 0x55, 0xFF, 0x00, 0xCC, 0x33]);
        editor.set_cursor_length(6);
        editor.bitwise_not();
        assert_eq!(editor.get_active_content(), &[0x55, 0xAA, 0x00, 0xFF, 0x33, 0xCC]);
        editor.set_cursor_length(3);
        editor.bitwise_not();
        assert_eq!(editor.get_active_content(), &[0xAA, 0x55, 0xFF, 0xFF, 0x33, 0xCC]);
    }

    #[test]
    fn test_replace_digit() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, &[0]);

        editor.set_active_converter(ConverterRef::HEX);
        editor.replace_digit('F');
        assert_eq!(editor.get_active_content(), &[0xF0]);
        editor.subcursor_next_digit();
        assert!(editor.replace_digit('Q').is_err());
        editor.replace_digit('1');
        assert_eq!(editor.get_active_content(), &[0xF1]);


        editor.set_active_converter(ConverterRef::BIN);
        editor.replace_digit('0');
        assert_eq!(editor.get_active_content(), &[0b01110001]);
        editor.subcursor_next_digit();
        editor.subcursor_next_digit();
        editor.subcursor_next_digit();
        editor.subcursor_next_digit();
        assert!(editor.replace_digit('2').is_err());
        editor.replace_digit('1');
        assert_eq!(editor.get_active_content(), &[0b01111001]);

        editor.set_active_converter(ConverterRef::DEC);
        editor.overwrite_bytes(0, &[0]);
        editor.replace_digit('2');
        assert_eq!(editor.get_active_content(), &[200]);
        editor.subcursor_next_digit();
        assert!(editor.replace_digit('6').is_err());
        editor.replace_digit('5');
        assert_eq!(editor.get_active_content(), &[250]);
    }

    #[test]
    fn test_parse_words() {
        let test_string = "word   one two \\\" \\  'double word' 'dub\\'two' \"b l a h\"";
        let assumption = [
            "word".to_string(),
            "one".to_string(),
            "two".to_string(),
            "\"".to_string(),
            " ".to_string(),
            "double word".to_string(),
            "dub'two".to_string(),
            "b l a h".to_string(),
        ];
        let words = parse_words(test_string);
        assert_eq!(words, assumption);

        assert_eq!(parse_words("\\x90"), &["\\x90".to_string()]);
    }

    #[test]
    fn test_string_to_integer() {
        assert_eq!(string_to_integer("12345"), Ok(12345));
        assert_eq!(string_to_integer("\\b1010"), Ok(10));
        assert_eq!(string_to_integer("\\x20"), Ok(32));
    }

    #[test]
    fn test_string_to_bytes() {
        assert_eq!(string_to_bytes("Test"), Ok("Test".as_bytes().to_vec()));
        assert_eq!(string_to_bytes("\\x90"), Ok(vec![0x90]));
        assert_eq!(string_to_bytes("\\b0100000010000000"), Ok(vec![64, 128]));
        assert_eq!(string_to_bytes("\\d16391"), Ok(vec![64, 7]));
    }

}
