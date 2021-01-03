use std::collections::{HashMap, HashSet};
use std::cmp::max;
use std::error::Error;
use wrecked::{RectManager, RectColor, RectError};

use super::sbyte_editor::*;
use super::sbyte_editor::converter::*;
use super::sbyte_editor::flag::Flag;


pub struct FrontEnd {
    rectmanager: RectManager,
    display_flags: HashMap<Flag, (usize, bool)>,
    display_flag_timeouts: HashMap<Flag, usize>,

    active_row_map: HashMap<usize, bool>,

    cells_to_refresh: HashSet<(usize, usize)>, // rect ids, rather than coords
    rows_to_refresh: Vec<usize>, // absolute row numbers
    active_cursor_cells: HashSet<(usize, usize)>, //rect ids of cells highlighted by cursor

    rect_display_wrapper: usize,
    rects_display: (usize, usize),
    rect_meta: usize,
    rect_offset: usize,
    rect_feedback: usize,

    last_known_viewport_offset: usize,

    row_dict: HashMap<usize, (usize, usize)>,
    cell_dict: HashMap<usize, HashMap<usize, (usize, usize)>>
}

impl FrontEnd {
    pub fn new() -> FrontEnd {
        let mut rectmanager = RectManager::new();
        let rect_display_wrapper = rectmanager.new_rect(wrecked::TOP).ok().unwrap();
        let id_display_bits = rectmanager.new_rect(rect_display_wrapper).ok().unwrap();
        let id_display_human = rectmanager.new_rect(rect_display_wrapper).ok().unwrap();
        let rect_meta = rectmanager.new_rect(wrecked::TOP).ok().unwrap();
        let rect_feedback = rectmanager.new_rect(rect_meta).ok().unwrap();
        let rect_offset = rectmanager.new_rect(rect_meta).ok().unwrap();

        let mut frontend = FrontEnd {
            rectmanager,
            active_row_map: HashMap::new(),
            cells_to_refresh: HashSet::new(),
            rows_to_refresh: Vec::new(),
            active_cursor_cells: HashSet::new(),

            rect_display_wrapper,
            rect_meta,
            rect_feedback,
            rect_offset,
            rects_display: (id_display_bits, id_display_human),
            row_dict: HashMap::new(),
            cell_dict: HashMap::new(),
            display_flags: HashMap::new(),
            display_flag_timeouts: HashMap::new(),
            last_known_viewport_offset: 9999
        };

        frontend.raise_flag(Flag::SetupDisplays);
        frontend.raise_flag(Flag::RemapActiveRows);

        frontend
    }

    pub fn tick(&mut self, sbyte_editor: &BackEnd) -> Result<(), Box::<dyn Error>> {
        if !sbyte_editor.is_loading() {

            if self.check_flag(Flag::SetupDisplays) {
                match self.setup_displays(sbyte_editor) {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::SetupFailed(error))?
                    }
                }
            }

            if self.check_flag(Flag::RemapActiveRows) {
                match self.remap_active_rows(sbyte_editor) {
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
                        match self.set_row_characters(sbyte_editor, y) {
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
                match self.apply_cursor(sbyte_editor) {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::ApplyCursorFailed(error))?
                    }
                }
            }


            match sbyte_editor.get_user_error_msg() {
                Some(msg) => {
                    self.display_user_error(msg.clone())?;
                }
                None => {
                    if self.check_flag(Flag::DisplayCMDLine) {
                        self.display_command_line(sbyte_editor)?;
                    } else {
                        match sbyte_editor.get_user_msg() {
                            Some(msg) => {
                                self.display_user_message(msg.clone())?;
                            }
                            None => {
                            }
                        }
                    }
                }
            }

            if self.check_flag(Flag::UpdateOffset) {
                self.display_user_offset(sbyte_editor)?;
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

    pub fn auto_resize(&mut self) -> bool {
        self.rectmanager.auto_resize()
    }

    pub fn raise_flag(&mut self, key: Flag) {
        match key {
            Flag::UpdateRow(some_y) => {
                self.rows_to_refresh.push(some_y)
            }
            _ => ()
        }

        self.display_flags.entry(key)
            .and_modify(|e| *e = (e.0, true))
            .or_insert((0, true));
    }

    //fn lower_flag(&mut self, key: Flag) {
    //    self.display_flags.entry(key)
    //        .and_modify(|e| *e = (e.0, false))
    //        .or_insert((0, false));
    //}

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

    fn remap_active_rows(&mut self, sbyte_editor: &BackEnd) -> Result<(), RectError> {
        let (width, height) = sbyte_editor.get_viewport_size();

        let initial_y = self.last_known_viewport_offset as isize;
        let new_y = (sbyte_editor.get_viewport_offset() / width) as isize;
        self.last_known_viewport_offset = new_y as usize;

        let diff: usize;
        if new_y > initial_y {
            diff = (new_y - initial_y) as usize;
        } else {
            diff = (initial_y - new_y) as usize;
        }

        let force_rerow = self.check_flag(Flag::ForceRerow);
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
                    self.raise_flag(Flag::UpdateRow(*y + (new_y as usize)));
                }
            }

            self.raise_flag(Flag::UpdateOffset);
        }

        self.raise_flag(Flag::ForceRerow);
        self.raise_flag(Flag::CursorMoved);

        Ok(())
    }

    fn setup_displays(&mut self, sbyte_editor: &BackEnd) -> Result<(), RectError> {
        // Assumes that the viewport size AND the rectmanager size are correctly set at this point
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();

        let (viewport_width, viewport_height) = sbyte_editor.get_viewport_size();

        self.rectmanager.resize(self.rect_meta, full_width, 1)?;
        self.rectmanager.resize(
            self.rect_feedback,
            full_width,
            1
        )?;

        self.rectmanager.set_position(self.rect_feedback, 0, 0)?;
        self.rectmanager.resize(
            self.rect_display_wrapper,
            full_width,
            full_height - 1
        )?;

        let (bits_display, human_display) = self.rects_display;
        self.rectmanager.clear_children(bits_display)?;
        self.rectmanager.clear_children(human_display)?;

        self.arrange_displays(sbyte_editor)?;

        self.cell_dict.drain();
        self.row_dict.drain();

        let display_ratio = sbyte_editor.get_display_ratio() as usize;
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

        self.raise_flag(Flag::ForceRerow);
        self.raise_flag(Flag::CursorMoved);

        Ok(())
    }

    fn arrange_displays(&mut self, sbyte_editor: &BackEnd) -> Result<(), RectError> {
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

        let display_ratio = sbyte_editor.get_display_ratio();
        let (vwidth, _vheight) = sbyte_editor.get_viewport_size();

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

    fn set_row_characters(&mut self, sbyte_editor: &BackEnd, absolute_y: usize) -> Result<(), RectError> {
        let human_converter = HumanConverter {};
        let active_converter = sbyte_editor.get_active_converter();
        let (width, _height) = sbyte_editor.get_viewport_size();
        let offset = width * absolute_y;

        let chunk = sbyte_editor.get_chunk(offset, width);
        let relative_y = absolute_y - (sbyte_editor.get_viewport_offset() / width);

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
                for (x, byte) in chunk.iter().enumerate() {
                    tmp_bits = active_converter.encode_byte(*byte);
                    tmp_human = human_converter.encode_byte(*byte);

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

    // TODO: Change this to use usize instead of BackEnd
    pub fn display_user_offset(&mut self, sbyte_editor: &BackEnd) -> Result<(), RectError> {
        let mut cursor_string = format!("{}", sbyte_editor.get_cursor_offset());
        let active_content = sbyte_editor.get_active_content();

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

        let cursor_len = sbyte_editor.get_cursor_length();
        let offset_display = if cursor_len == 1 {
                format!("Offset: {} / {}", cursor_string, denominator)
            } else {
                format!("Offset: {} ({}) / {}", cursor_string, cursor_len, denominator)

            };

        let meta_width = self.rectmanager.get_rect_width(self.rect_meta);

        let x = meta_width - offset_display.len();
        self.rectmanager.resize(self.rect_offset, offset_display.len(), 1)?;
        self.rectmanager.set_position(self.rect_offset, x as isize, 0)?;
        self.rectmanager.set_string(self.rect_offset, 0, 0, &offset_display)?;

        Ok(())
    }

    pub fn display_user_message(&mut self, msg: String) -> Result<(), RectError> {
        self.clear_feedback()?;

        self.rectmanager.set_string(self.rect_feedback, 0, 0, &msg)?;
        self.rectmanager.set_bold_flag(self.rect_feedback)?;
        self.rectmanager.set_fg_color(self.rect_feedback, RectColor::BRIGHTCYAN)?;

        Ok(())
    }

    pub fn display_user_error(&mut self, msg: String) -> Result<(), RectError> {
        self.clear_feedback()?;
        self.rectmanager.set_string(self.rect_feedback, 0, 0, &msg)?;
        self.rectmanager.set_fg_color(self.rect_feedback, RectColor::RED)?;

        Ok(())
    }

    fn clear_feedback(&mut self) -> Result<(), RectError> {
        self.rectmanager.clear_characters(self.rect_feedback)?;
        self.rectmanager.clear_children(self.rect_feedback)?;
        self.rectmanager.unset_bold_flag(self.rect_feedback)?;
        self.rectmanager.unset_fg_color(self.rect_feedback)?;

        Ok(())
    }

    pub fn apply_cursor(&mut self, sbyte_editor: &BackEnd) -> Result<(), RectError> {
        let (viewport_width, viewport_height) = sbyte_editor.get_viewport_size();
        let viewport_offset = sbyte_editor.get_viewport_offset();
        let cursor_offset = sbyte_editor.get_cursor_offset();
        let cursor_length = sbyte_editor.get_cursor_length();

        // First clear previously applied
        // (They may no longer exist, but that's ok)
        for (bits, human) in self.active_cursor_cells.drain() {
            self.rectmanager.unset_invert_flag(bits).ok();
            self.rectmanager.unset_invert_flag(human).ok();
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

    pub fn display_command_line(&mut self, sbyte_editor: &BackEnd) -> Result<(), RectError> {
        match sbyte_editor.get_commandline() {
            Some(commandline) => {
                self.clear_feedback()?;


                let cmd = &commandline.get_register();
                // +1, because of the ":" at the start
                let cursor_x = commandline.get_cursor_offset() + 1;
                let cursor_id = self.rectmanager.new_rect(self.rect_feedback).ok().unwrap();

                self.rectmanager.resize(cursor_id, 1, 1)?;
                self.rectmanager.set_position(cursor_id, cursor_x as isize, 0)?;
                self.rectmanager.set_invert_flag(cursor_id)?;

                if cursor_x < cmd.len() {
                    let chr: String = cmd.chars().skip(cursor_x).take(1).collect();
                    self.rectmanager.set_string(cursor_id, 0, 0, &chr)?;
                }

                self.rectmanager.set_string(self.rect_feedback, 0, 0, &vec![":", cmd].join(""))?;
            }
            None => {
            }
        }

        Ok(())
    }

    pub fn size(&self) -> (usize, usize) {
        let width = self.rectmanager.get_width();
        let height = self.rectmanager.get_height();
        (width, height)
    }

    pub fn get_viewport_height(&self) -> usize {
       self.size().1 - self.get_meta_height()
    }

    pub fn get_meta_height(&self) -> usize {
        1
    }

    pub fn kill(&mut self) -> Result<(), SbyteError> {
        match self.rectmanager.kill() {
            Ok(_) => {
                Ok(())
            }
            Err(_e) => {
                Err(SbyteError::FailedToKill)
            }
        }
    }
}

