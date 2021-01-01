#[cfg (test)]
mod tests {
    use crate::sbyte_editor::BackEnd;
    #[test]
    fn test_initializes_empty() {
        let editor = BackEnd::new();
        assert_eq!(editor.active_content.as_slice(), []);
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
        editor.insert_bytes(0, vec![65, 66, 0, 0, 65, 65, 66, 65]);

        let found = editor.find_all("AB").ok().unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, 0);
        assert_eq!(found[1].0, 5);

        assert_eq!(editor.find_after("AB", 2).ok().unwrap(), Some((5, 7)));

    }
}
