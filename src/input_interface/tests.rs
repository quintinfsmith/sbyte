#[cfg (test)]
mod tests {
    use crate::input_interface::Inputter;
    use std::{time, thread};

    #[test]
    fn test_inputter() {
        let mut inputter = Inputter::new();
        inputter.assign_mode_command("DEFAULT", b"Q", "TEST", &[]);
        assert_eq!(inputter.active_node, 0);
        assert_eq!(inputter.input_nodes.len(), 2);
        inputter.input(b"Q"[0]);
        assert_eq!(inputter.fetch_hook(), Some(("TEST".to_string(), Vec::new())));
        assert_eq!(inputter.active_node, 0);


        inputter.assign_mode_command("DEFAULT", b"ABCD", "TEST2", &[]);
        inputter.input(b'A');
        inputter.input(b'B');
        inputter.input(b'C');
        assert_eq!(inputter.fetch_hook(), None);
        assert_eq!(inputter.active_node, 4);
        inputter.input(b'D');
        assert_eq!(inputter.fetch_hook(), Some(("TEST2".to_string(), Vec::new())));

        inputter.input(b'A');
        inputter.input(b'B');
        inputter.input(b'C');
        assert_eq!(inputter.fetch_hook(), None);
        inputter.input(b'C');
        assert_eq!(inputter.fetch_hook(), None);
        assert_eq!(inputter.active_node, 0);
    }


}
