#[cfg (test)]
mod tests {
    use crate::sbyte_editor::{BackEnd, ConverterRef, HexConverter, BinaryConverter, DecConverter};

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

        editor.insert_bytes(0, vec![65]);
        assert_eq!(editor.active_content.as_slice(), [65]);

        // inserting out of range should ignore insertion
        editor.insert_bytes(10, vec![65]);
        assert_eq!(editor.active_content.as_slice(), [65]);
    }

    #[test]
    fn test_remove_bytes() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, vec![65]);

        assert_eq!(editor.remove_bytes(0, 1), vec![65]);
        assert_eq!(editor.active_content.as_slice(), []);
        assert_eq!(editor.remove_bytes(1000, 300), vec![]);

    }
    #[test]
    fn test_remove_bytes_at_cursor() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, vec![65]);
        editor.set_cursor_offset(0);
        assert_eq!(editor.remove_bytes_at_cursor(), vec![65]);
        assert_eq!(editor.active_content.as_slice(), []);
    }

    #[test]
    fn test_yanking() {
        let mut editor = BackEnd::new();

        editor.insert_bytes(0, vec![65, 66, 67, 68]);

        editor.make_selection(1, 3);
        assert_eq!(editor.get_selected().as_slice(), [66, 67, 68]);

        editor.copy_selection();
        assert_eq!(editor.get_clipboard().as_slice(), [66, 67, 68]);
    }

    #[test]
    fn test_find() {
        let mut editor = BackEnd::new();
        editor.insert_bytes(0, vec![65, 66, 0, 0, 65, 65, 66, 60, 0x30]);
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
            vec![0xFF, 0x0F, 0xF0, 0x77, 0x7F, 0xF7, 0xC0, 0x0C]
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
        assert!(editor.increment_byte(0).is_err(), "Didn't throw error when incrementing outside of valid range.");

        editor.insert_bytes(0, vec![0]);
        assert!(editor.increment_byte(0).is_ok(), "Failed to increment byte");
        assert_eq!(
            editor.undo_stack.last(),
            Some(&(0,1, vec![0])),
            "Undo stack wasn't properly pushed to"
        );
    }

    #[test]
    fn test_decrement_byte() {
        let mut editor = BackEnd::new();
        assert!(editor.decrement_byte(0).is_err(), "Didn't throw error when decrementing outside of valid range.");

        editor.insert_bytes(0, vec![1]);
        assert!(editor.decrement_byte(0).is_ok(), "Failed to decrement byte");
        assert_eq!(
            editor.undo_stack.last(),
            Some(&(0,1, vec![1])),
            "Undo stack wasn't properly pushed to"
        );
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

        editor.insert_bytes(0, vec![0]);

        assert!(editor.undo().is_ok(), "Failed to undo");
    }

    #[test]
    fn test_redo() {
        let mut editor = BackEnd::new();

        editor.insert_bytes(0, vec![0]);
        assert!(editor.redo().is_err(), "Didn't raise error when trying to redo from an (presumably) empty stack");
        editor.undo();

        assert!(editor.redo().is_ok(), "Failed to redo");
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
}
