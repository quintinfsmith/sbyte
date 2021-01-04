#[cfg (test)]
mod tests {
    use crate::sbyte_editor::content::Content;

    #[test]
    fn test_initialize() {
        let mut content = Content::new();
        assert_eq!(content.as_slice(), []);
    }

    #[test]
    fn test_insert_bytes() {
        let mut content = Content::new();

        assert!(content.insert_bytes(1, [99].to_vec()).is_err());
        assert!(content.insert_bytes(0, [34,35,36,37].to_vec()).is_ok());
        assert_eq!(content.as_slice(), [34, 35, 36, 37]);

        assert!(content.insert_bytes(2, [0, 0, 0].to_vec()).is_ok());
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
        content.insert_bytes(0, slice.to_vec());
        for (i, byte) in slice.iter().enumerate() {
            assert_eq!(content.get_byte(i), *byte);
        }
    }

    #[test]
    fn test_get_chunk() {
        let mut content = Content::new();
        let slice = [45,46,47,23,12];
        content.insert_bytes(0, slice.to_vec());
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
        content.insert_bytes(0, [34,35,36,37].to_vec());
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
        content.insert_bytes(0, slice.to_vec());

        assert_eq!(content.as_slice(), slice);
    }

    #[test]
    fn test_find_all() {
        let mut content = Content::new();
        let mut slice = [0x90, 0x91, 0x80, 0x80, 0x90, 0x90, 0x90, 0x90];
        content.insert_bytes(0, slice.to_vec());

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
}
