use std::collections::{HashMap, HashSet};
use std::cmp::{min, max};
use std::fmt;

use wrecked::{RectManager, RectColor, RectError};

use super::sbyte_editor;

impl InConsole for SbyteEditor {
    fn flag_row_update_by_range(&mut self, range: std::ops::Range<usize>) {
        let viewport_width = self.viewport.get_width();
        let first_active_row = range.start / viewport_width;
        let last_active_row = range.end / viewport_width;

        for y in first_active_row .. max(last_active_row + 1, first_active_row + 1) {
            self.raise_flag(Flag::UpdateRow(y));
            self.raise_row_update_flag(y);
        }
    }

    fn flag_row_update_by_offset(&mut self, offset: usize) {
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();
        let first_active_row = offset / viewport_width;

        for y in first_active_row .. first_active_row + viewport_height {
            self.raise_row_update_flag(y);
        }
    }

    fn raise_row_update_flag(&mut self, absolute_y: usize) {
        self.raise_flag(Flag::UpdateRow(absolute_y));
        self.rows_to_refresh.push(absolute_y);
    }

    fn raise_flag(&mut self, key: Flag) {
        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, true))
            .or_insert((0, true));
    }

    fn lower_flag(&mut self, key: Flag) {
        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, false))
            .or_insert((0, false));
    }
}

pub struct ConsoleSbyte {
    sbyte_editor: SbyteEditor,
    rectmanager: RectManager,

    active_row_map: HashMap<usize, bool>,
    locked_viewport_width: Option<usize>,

    cells_to_refresh: HashSet<(usize, usize)>, // rect ids, rather than coords
    rows_to_refresh: Vec<usize>, // absolute row numbers
    active_cursor_cells: HashSet<(usize, usize)>, //rect ids of cells highlighted by cursor

    is_resizing: bool,

    rect_display_wrapper: usize,
    rects_display: (usize, usize),
    rect_meta: usize,

    row_dict: HashMap<usize, (usize, usize)>,
    cell_dict: HashMap<usize, HashMap<usize, (usize, usize)>>
}

impl ConsoleSbyte {
    fn new() -> ConsoleSbyte {
        let mut rectmanager = RectManager::new();
        let (width, height) = rectmanager.get_rect_size(wrecked::TOP).unwrap();
        let rect_display_wrapper = rectmanager.new_rect(wrecked:TOP).ok().unwrap();
        let id_display_bits = rectmanager.new_rect(id_display_wrapper).ok().unwrap();
        let id_display_human = rectmanager.new_rect(id_display_wrapper).ok().unwrap();
        let rect_meta = rectmanager.new_rect(wrecked::TOP).ok().unwrap();


        ConsoleSbyte {
            rectmanager,
            sbyte_editor: SbyteEditor::new(),
            active_row_map: HashMap::new(),
            cells_to_refresh: HashSet::new(),
            rows_ro_refresh: Vec::new(),
            active_cursor_cells: HashSet::new(),

            rect_display_wrapper,
            rect_meta,
            rects_display: (id_display_bits, id_display_human),
            row_dict: HashMap::new(),
            cell_dict: HashMap::new()
        }
    }

    fn tick(&mut self) -> Result<(), Box::<dyn Error>> {
        if !self.sbyte_editor.is_loading() {
            self.check_resize();

            if self.check_flag(Flag::SetupDisplays) {
                match self.setup_displays() {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::SetupFailed(error))?
                    }
                }
            }

            if self.check_flag(Flag::RemapActiveRows) {
                match self.remap_active_rows() {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::RemapFailed(error))?
                    }
                }
            }

            let len = self.rows_to_refresh.len();
            if len > 0 {
                let mut in_timeout = Vec::new();
                let mut y;
                while self.rows_to_refresh.len() > 0 {
                    y = self.rows_to_refresh.pop().unwrap();
                    if self.check_flag(Flag::UpdateRow(y)) {
                        match self.set_row_characters(y) {
                            Ok(_) => {}
                            Err(error) => {
                                Err(SbyteError::RowSetFailed(error))?
                            }
                        }
                    } else {
                        in_timeout.push(y);
                    }
                }
                self.rows_to_refresh = in_timeout;
            }


            if self.check_flag(Flag::CursorMoved) {
                match self.apply_cursor() {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::ApplyCursorFailed(error))?
                    }
                }
            }


            match self.user_error_msg.clone() {
                Some(msg) => {
                    self.display_user_error(msg.clone())?;
                    self.user_error_msg = None;

                    // Prevent any user msg from clobbering this msg
                    self.user_msg = None;
                    self.lower_flag(Flag::UpdateOffset);
                }
                None => {
                    if self.check_flag(Flag::DisplayCMDLine) {
                        self.display_command_line()?;
                    } else {
                        let tmp_usr_msg = self.user_msg.clone();
                        match tmp_usr_msg {
                            Some(msg) => {
                                self.display_user_message(msg.clone())?;
                                self.user_msg = None;
                                self.lower_flag(Flag::UpdateOffset);
                            }
                            None => {
                                if self.check_flag(Flag::UpdateOffset) {
                                    self.display_user_offset()?;
                                }
                            }
                        }
                    }
                }
            }

            match self.rectmanager.draw() {
                Ok(_) => {}
                Err(error) => {
                    Err(SbyteError::DrawFailed(error))?;
                }
            }
        }

        Ok(())
    }

    fn auto_resize(&mut self) -> bool {
        self.rectmanager.auto_resize()
    }

    fn raise_row_update_flag(&mut self, absolute_y: usize) {

        self.raise_flag(Flag::UpdateRow(absolute_y));
        self.rows_to_refresh.push(absolute_y);
    }

    fn raise_flag(&mut self, key: Flag) {
        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, true))
            .or_insert((0, true));
    }

    fn lower_flag(&mut self, key: Flag) {
        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, false))
            .or_insert((0, false));
    }

    fn check_flag(&mut self, key: Flag) -> bool {
        let mut output = false;
        match self.display_flags.get_mut(&key) {
            Some((countdown, flagged)) => {
                if *countdown > 0 {
                    *countdown -= 1;
                } else {
                    output = *flagged;
                }
            }
            None => ()
        }

        if output {
            let mut new_timeout = 0;
            match self.display_flag_timeouts.get(&key) {
                Some(timeout) => {
                    new_timeout = *timeout;
                }
                None => { }
            }

            self.display_flags.entry(key)
                .and_modify(|e| *e = (new_timeout, false))
                .or_insert((new_timeout, false));
        }

        output
    }

    fn remap_active_rows(&mut self) -> Result<(), RectError> {
        let (width, _height) = self.sbyte_editor.get_viewport_size();
        let initial_y = (self.sbyte_editor.get_viewport_offset() / width) as isize;

        self.sbyte_editor.adjust_viewport_offset();
        let new_y = (self.sbyte_editor.get_viewport_offset() / width) as isize;

        let diff: usize;
        if new_y > initial_y {
            diff = (new_y - initial_y) as usize;
        } else {
            diff = (initial_y - new_y) as usize;
        }

        let force_rerow = self.sbyte_editor.check_flag(Flag::ForceRerow);
        if diff > 0 || force_rerow {
            if diff < height && !force_rerow {
                // Don't rerender rendered rows. just shuffle them around
                {
                    let (bits, human) = self.rects_display;
                    self.rectmanager.shift_contents(
                        human,
                        0,
                        initial_y - new_y
                    )?;
                    self.rectmanager.shift_contents(
                        bits,
                        0,
                        initial_y - new_y
                    )?;
                }

                let mut new_rows_map = HashMap::new();
                let mut new_cells_map = HashMap::new();
                let mut new_active_map = HashMap::new();
                let mut from_y;
                if new_y < initial_y {
                    // Reassign the display_dicts to correspond to correct rows
                    for y in 0 .. height {

                        if diff > y {
                            from_y = height - ((diff - y) % height);
                        } else {
                            from_y = (y - diff) % height;
                        }

                        match self.row_dict.get(&from_y) {
                            Some((bits, human)) => {
                                new_rows_map.entry(y)
                                    .and_modify(|e| { *e = (*bits, *human)})
                                    .or_insert((*bits, *human));
                            }
                            None => ()
                        }

                        match self.cell_dict.get(&from_y) {
                            Some(cellhash) => {
                                new_cells_map.entry(y)
                                    .and_modify(|e| { *e = cellhash.clone()})
                                    .or_insert(cellhash.clone());
                            }
                            None => ()
                        }

                        if y < from_y {
                            // Moving row at bottom to top
                            match new_rows_map.get(&y) {
                                Some((bits, human)) => {
                                    self.rectmanager.set_position(*bits, 0, y as isize)?;
                                    self.rectmanager.set_position(*human, 0, y as isize)?;
                                }
                                None => ()
                            }
                            new_active_map.insert(y, false);
                        } else {
                            match self.active_row_map.get(&from_y) {
                                Some(needs_refresh) => {
                                    new_active_map.insert(y, *needs_refresh);
                                }
                                None => ()
                            }
                        }
                    }
                } else {
                    for y in 0 .. height {
                        from_y = (y + diff) % height;
                        match self.row_dict.get(&from_y) {
                            Some((bits, human)) => {
                                new_rows_map.entry(y)
                                    .and_modify(|e| { *e = (*bits, *human)})
                                    .or_insert((*bits, *human));

                            }
                            None => ()
                        }

                        match self.cell_dict.get(&from_y) {
                            Some(cellhash) => {
                                new_cells_map.entry(y)
                                    .and_modify(|e| { *e = cellhash.clone()})
                                    .or_insert(cellhash.clone());
                            }
                            None => ()
                        }

                        if from_y < y {
                            //Moving row at top to the bottom
                            match new_rows_map.get(&y) {
                                Some((bits, human)) => {
                                    self.rectmanager.set_position(*human, 0, y as isize)?;
                                    self.rectmanager.set_position(*bits, 0, y as isize)?;
                                }
                                None => ()
                            }
                            new_active_map.insert(y, false);
                        } else {
                            match self.active_row_map.get(&from_y) {
                                Some(needs_refresh) => {
                                    new_active_map.insert(y, *needs_refresh);
                                }
                                // *Shouldn't* happen
                                None => {
                                    new_active_map.insert(y, false);
                                }
                            }
                        }
                    }
                }

                self.active_row_map = new_active_map;
                for (y, (bits, human)) in new_rows_map.iter() {
                    self.row_dict.entry(*y)
                        .and_modify(|e| {*e = (*bits, *human)})
                        .or_insert((*bits, *human));
                }

                for (y, cells) in new_cells_map.iter() {
                    self.cell_dict.entry(*y)
                        .and_modify(|e| {*e = cells.clone()})
                        .or_insert(cells.clone());
                }
            } else {
                self.active_row_map.drain();
                for y in 0 .. height {
                    self.active_row_map.insert(y, false);
                }
            }

            let active_rows = self.active_row_map.clone();
            for (y, is_rendered) in active_rows.iter() {
                if !is_rendered {
                    self.sbyte_editor.raise_row_update_flag(*y + (new_y as usize));
                }
            }

            self.sbyte_editor.raise_flag(Flag::UpdateOffset);
        }

        self.sbyte_editor.raise_flag(Flag::ForceRerow);
        self.sbyte_editor.raise_flag(Flag::CursorMoved);

        Ok(())
    }

    fn autoset_viewport_size(&mut self) {
        let full_height = self.rectmanager.get_height();
        let full_width = self.rectmanager.get_width();
        let meta_height = 1;

        let display_ratio = self.sbyte_editor.get_display_ratio() as f64;
        let r: f64 = 1f64 / display_ratio;
        let a: f64 = 1f64 - ( 1f64 / (r + 1f64));
        let mut base_width = ((full_width as f64) * a) as usize;

        // If the editor will overflow the new viewport with a locked viewport width, unlock it.
        let do_unlock = match self.sbyte_editor.get_locked_viewport_width() {
            Some(locked_width) => {
                locked_width > base_width
            }
            None => {
                false
            }
        };
        if do_unlock {
            self.sbyte_editor.unlocked_viewport_width();
        }


        self.sbyte_editor.set_viewport_size(
            base_width,
            full_height - meta_height
        );

        self.active_row_map.drain();
        for i in 0 .. self.viewport.get_height() {
            self.active_row_map.insert(i, false);
        }
    }

    fn setup_displays(&mut self) -> Result<(), RectError> {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();

        self.autoset_viewport_size();
        let viewport_width = self.viewport.get_width();
        let viewport_height = self.viewport.get_height();

        self.rectmanager.resize(self.rect_meta, full_width, 1)?;
        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            full_height - 1
        )?;

        let (bits_display, human_display) = self.rects_display;
        self.rectmanager.clear_children(bits_display)?;
        self.rectmanager.clear_children(human_display)?;

        self.arrange_displays()?;

        self.cell_dict.drain();
        self.row_dict.drain();

        let display_ratio = self.sbyte_editor.get_display_ratio() as usize;
        let width_bits;
        if display_ratio != 1 {
            width_bits = max(1, display_ratio - 1);
        } else {
            width_bits = display_ratio;
        }

        let mut _bits_row_id;
        let mut _bits_cell_id;
        let mut _human_row_id;
        let mut _human_cell_id;
        let mut _cells_hashmap;
        for y in 0..viewport_height {
            self.active_row_map.entry(y)
                .and_modify(|e| *e = false)
                .or_insert(false);

            _bits_row_id = self.rectmanager.new_rect(bits_display).ok().unwrap();

            self.rectmanager.resize(
                _bits_row_id,
                (viewport_width * display_ratio) - 1,
                1
            )?;

            self.rectmanager.set_position(_bits_row_id, 0, y as isize)?;

            _human_row_id = self.rectmanager.new_rect(human_display).ok().unwrap();
            self.rectmanager.resize(
                _human_row_id,
                viewport_width,
                1
            )?;
            self.rectmanager.set_position(
                _human_row_id,
                0,
                y as isize
            )?;

            self.row_dict.entry(y)
                .and_modify(|e| *e = (_bits_row_id, _human_row_id))
                .or_insert((_bits_row_id, _human_row_id));

            _cells_hashmap = self.cell_dict.entry(y).or_insert(HashMap::new());

            for x in 0 .. viewport_width {
                _bits_cell_id = self.rectmanager.new_rect(_bits_row_id).ok().unwrap();
                self.rectmanager.resize(
                    _bits_cell_id,
                    width_bits,
                    1
                )?;

                self.rectmanager.set_position(
                    _bits_cell_id,
                    (x * display_ratio) as isize,
                    0
                )?;

                _human_cell_id = self.rectmanager.new_rect(_human_row_id).ok().unwrap();

                self.rectmanager.set_position(
                    _human_cell_id,
                    x as isize,
                    0
                )?;
                self.rectmanager.resize(_human_cell_id, 1, 1)?;

                _cells_hashmap.entry(x as usize)
                    .and_modify(|e| *e = (_bits_cell_id, _human_cell_id))
                    .or_insert((_bits_cell_id, _human_cell_id));
            }
        }

        self.sbyte_editor.raise_flag(Flag::ForceRerow);

        self.sbyte_editor.raise_flag(Flag::CursorMoved);

        Ok(())
    }

    fn arrange_displays(&mut self) -> Result<(), RectError> {
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        let meta_height = 1;

        self.rectmanager.set_position(
            self.rect_meta,
            0,
            (full_height - meta_height) as isize
        )?;


        let display_height = full_height - meta_height;
        self.rectmanager.clear_characters(self.rect_display_wrapper)?;

        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            display_height
        )?;

        self.rectmanager.set_position(
            self.rect_display_wrapper,
            0,
            0
        )?;

        let display_ratio = self.sbyte_editor.get_display_ratio();
        let (vwidth, vheight) = self.sbyte_editor.get_viewport_size();
        let (bits_id, human_id) = self.rects_display;
        let human_display_width = vwidth;
        let bits_display_width = vwidth * display_ratio as usize;
        let remaining_space = full_width - bits_display_width - human_display_width;


        let bits_display_x = remaining_space / 2;

        self.rectmanager.resize(bits_id, bits_display_width, display_height)?;
        self.rectmanager.set_position(bits_id, bits_display_x as isize, 0)?;

        // TODO: Fill in a separator

        let human_display_x = (remaining_space / 2) + bits_display_width;

        self.rectmanager.resize(human_id, human_display_width, display_height)?;
        self.rectmanager.set_position(human_id, human_display_x as isize, 0)?;

        Ok(())
    }

    fn set_row_characters(&mut self, absolute_y: usize) -> Result<(), RectError> {
        let human_converter = HumanConverter {};
        let active_converter = self.sbyte_editor.get_active_converter();
        let (width, _height) = self.sbyte_editor.get_width();
        let offset = width * absolute_y;

        let structure_handlers = self.sbyte_editor.get_visible_structured_data_handlers(offset, width);
        let mut structured_cells_map = HashMap::new();
        let mut x;
        let mut y;
        for (span, sid) in structure_handlers.iter() {
            for i in span.0 .. span.1 {
                x = i % width;
                y = i / width;
                structured_cells_map.entry((x, y)).or_insert(self.structure_validity[sid]);
            }
        }

        let chunk = self.sbyte_editor.get_chunk(offset, width);
        let relative_y = absolute_y - (self.sbyte_editor.get_viewport_offset() / width);

        match self.cell_dict.get_mut(&relative_y) {
            Some(cellhash) => {

                for (_x, (rect_id_bits, rect_id_human)) in cellhash.iter_mut() {
                    self.rectmanager.clear_characters(*rect_id_human)?;
                    self.rectmanager.clear_characters(*rect_id_bits)?;
                }

                let mut tmp_bits;
                let mut tmp_bits_str;
                let mut tmp_human;
                let mut tmp_human_str;
                let mut in_structure;
                let mut structure_valid;
                for (x, byte) in chunk.iter().enumerate() {
                    tmp_bits = active_converter.encode_byte(*byte);
                    tmp_human = human_converter.encode_byte(*byte);

                    match structured_cells_map.get(&(x, absolute_y)) {
                        Some(is_valid) => {
                            in_structure = true;
                            structure_valid = *is_valid;
                        }
                        None => {
                            structure_valid = false;
                            in_structure = false;
                        }
                    }

                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            tmp_bits_str = match std::str::from_utf8(tmp_bits.as_slice()) {
                                Ok(valid) => {
                                    valid
                                }
                                Err(_) => {
                                    // Shouldn't Happen
                                    "."
                                }
                            };
                            tmp_human_str = match std::str::from_utf8(tmp_human.as_slice()) {
                                Ok(valid) => {
                                    valid
                                }
                                Err(_) => {
                                    "."
                                }
                            };

                            for (i, c) in tmp_human_str.chars().enumerate() {
                                self.rectmanager.set_character(*human, i as isize, 0, c)?;
                            }
                            for (i, c) in tmp_bits_str.chars().enumerate() {
                                self.rectmanager.set_character(*bits, i as isize, 0, c)?;
                            }

                            if in_structure {
                                self.rectmanager.set_underline_flag(*human)?;
                                self.rectmanager.set_underline_flag(*bits)?;
                            } else {
                                self.rectmanager.unset_underline_flag(*human)?;
                                self.rectmanager.unset_underline_flag(*bits)?;
                            }

                            if in_structure && !structure_valid {
                                self.rectmanager.set_fg_color(*human, RectColor::RED)?;
                                self.rectmanager.set_fg_color(*bits, RectColor::RED)?;
                            } else {
                                self.rectmanager.unset_color(*human)?;
                                self.rectmanager.unset_color(*bits)?;
                            }

                        }
                        None => { }
                    }
                }
            }
            None => { }
        }

        self.active_row_map.entry(relative_y)
            .and_modify(|e| {*e = true})
            .or_insert(true);

        Ok(())
    }

    fn display_user_offset(&mut self) -> Result<(), RectError> {
        let mut cursor_string = format!("{}", self.sbyte_editor.get_cursor_offset());
        let mut active_content = self.sbyte_editor.get_active_content();

        if active_content.len() > 0 {
            let digit_count = (active_content.len() as f64).log10().ceil() as usize;
            let l = cursor_string.len();
            if l < digit_count {
                for _ in 0 .. (digit_count - l) {
                    cursor_string = format!("{}{}", " ", cursor_string);
                }
            }

        }

        let denominator = if active_content.len() == 0 {
            0
        } else {
            active_content.len() - 1
        };

        let cursor_len = self.sbyte_editor.get_cursor_length();
        let offset_display = if cursor_len == 1 {
                format!("Offset: {} / {}", cursor_string, denominator)
            } else {
                format!("Offset: {} ({}) / {}", cursor_string, cursor_len, denominator)

            };

        let meta_width = self.rectmanager.get_rect_width(self.rect_meta);

        let x = meta_width - offset_display.len();

        self.clear_meta_rect()?;

        self.rectmanager.set_string(self.rect_meta, x as isize, 0, &offset_display)?;

        Ok(())
    }

    fn clear_meta_rect(&mut self) -> Result<(), RectError> {
        self.rectmanager.clear_characters(self.rect_meta)?;
        self.rectmanager.clear_children(self.rect_meta)?;
        self.rectmanager.clear_effects(self.rect_meta)?;

        Ok(())
    }

    fn display_user_message(&mut self, msg: String) -> Result<(), RectError> {
        self.clear_meta_rect()?;
        self.rectmanager.set_string(self.rect_meta, 0, 0, &msg)?;
        self.rectmanager.set_bold_flag(self.rect_meta)?;
        self.rectmanager.set_fg_color(self.rect_meta, RectColor::BRIGHTCYAN)?;

        Ok(())
    }

    fn display_user_error(&mut self, msg: String) -> Result<(), RectError> {
        self.clear_meta_rect()?;
        self.rectmanager.set_string(self.rect_meta, 0, 0, &msg)?;
        self.rectmanager.set_fg_color(self.rect_meta, RectColor::RED)?;

        Ok(())
    }

    fn apply_cursor(&mut self) -> Result<(), RectError> {
        let (viewport_width, viewport_height) = self.sbyte_editor.get_viewport_size();
        let viewport_offset = self.sbyte_editor.get_viewport_offset();
        let cursor_offset = self.sbyte_editor.cursor_offset();
        let cursor_length = self.sbyte_editor.cursor_length();

        // First clear previously applied
        // (They may no longer exist, but that's ok)
        for (bits, human) in self.active_cursor_cells.drain() {
            self.rectmanager.unset_invert_flag(bits)?;
            self.rectmanager.unset_invert_flag(human)?;
        }

        let start = if cursor_offset < viewport_offset {
            viewport_offset
        } else {
            cursor_offset
        };

        let end = if cursor_offset + cursor_length > viewport_offset + (viewport_height * viewport_width) {
            viewport_offset + (viewport_height * viewport_width)
        } else {
            cursor_offset + cursor_length
        };

        let mut y;
        let mut x;
        for i in start .. end {
            y = (i - viewport_offset) / viewport_width;
            match self.cell_dict.get(&y) {
                Some(cellhash) => {
                    x = (i - viewport_offset) % viewport_width;
                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            self.rectmanager.set_invert_flag(*bits)?;
                            self.rectmanager.set_invert_flag(*human)?;
                            self.cells_to_refresh.insert((*bits, *human));
                            self.active_cursor_cells.insert((*bits, *human));
                        }
                        None => ()
                    }
                }
                None => ()
            }
        }

        Ok(())
    }

    fn display_command_line(&mut self) -> Result<(), RectError> {
        self.clear_meta_rect()?;
        match self.sbyte_editor.get_commandline() {
            Some(commandline) => {
                let cmd = &commandline.get_register();
                // +1, because of the ":" at the start
                let cursor_x = commandline.get_cursor_offset() + 1;
                let cursor_id = self.rectmanager.new_rect(self.rect_meta).ok().unwrap();

                self.rectmanager.resize(cursor_id, 1, 1)?;
                self.rectmanager.set_position(cursor_id, cursor_x as isize, 0)?;
                self.rectmanager.set_invert_flag(cursor_id)?;

                if cursor_x < cmd.len() {
                    let chr: String = cmd.chars().skip(cursor_x).take(1).collect();
                    self.rectmanager.set_string(cursor_id, 0, 0, &chr)?;
                }

                self.rectmanager.set_string(self.rect_meta, 0, 0, &vec![":", cmd].join(""))?;
            }
            None => {
            }
        }

        Ok(())
    }

    pub fn main(&mut self) -> Result<(), Box<dyn Error>> {
        let input_interface: Arc<Mutex<InputterEditorInterface>> = Arc::new(Mutex::new(InputterEditorInterface::new()));

        // Catch the Ctrl+C Signal
        let signal_mutex = input_interface.clone();
        ctrlc::set_handler(move || {
            let mut ok = false;
            while !ok {
                match signal_mutex.try_lock() {
                    Ok(ref mut mutex) => {
                        mutex.flag_kill = true;
                        ok = true;
                    }
                    Err(_e) => ()
                }
            }
        }).expect("Error setting Ctrl-C handler");

        self.sbyte_editor.spawn_input_daemon(input_interface.clone());
        self.sbyte_editor.spawn_input_processor_daemon(input_interface.clone());


        let fps = 59.97;
        let nano_seconds = ((1f64 / fps) * 1_000_000_000f64) as u64;
        let delay = time::Duration::from_nanos(nano_seconds);
        while !self.sbyte_editor.killed() {
            match self.tick() {
                Ok(_) => {
                    thread::sleep(delay);
                }
                Err(boxed_error) => {
                    // To help debug ...
                    self.sbyte_editor.set_user_error_msg(format!("{:?}", boxed_error));
                    //self.flag_kill = true;
                    //Err(Box::new(error))?;
                }
            }
        }

        self.kill()?;

        Ok(())
    }
}

