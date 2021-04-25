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
        inputter.go_to_next(b"Q"[0]);
        assert_eq!(inputter.active_node, 1);

        assert_eq!(inputter.fetch_hook(), Some(("TEST".to_string(), Vec::new())));
        assert_eq!(inputter.active_node, 0);
    }


}
