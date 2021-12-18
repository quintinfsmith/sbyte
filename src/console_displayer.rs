use std::collections::{HashMap, HashSet};
use std::cmp::max;
use std::error::Error;
use wrecked::{RectManager, Color, WreckedError};

use super::shell::Shell;
use super::editor::*;
use super::editor::formatter::*;
use std::time::{Duration, Instant};
use std::{time, thread};

use usize as RectId;

pub struct FrontEnd {
    rectmanager: RectManager,

    active_row_map: HashMap<usize, bool>,

    cells_to_refresh: HashSet<(RectId, RectId)>, // rect ids, rather than coords
    rows_to_refresh: HashSet<usize>, // absolute row numbers
    active_cursor_cells: HashSet<(RectId, RectId)>, //rect ids of cells highlighted by cursor

    rect_display_wrapper: RectId,
    rects_display: (RectId, RectId),
    rect_meta: RectId,
    rect_offset: RectId,
    rect_feedback: RectId,
    rect_scrollbar: RectId,


    row_dict: HashMap<usize, (RectId, RectId)>,
    cell_dict: HashMap<usize, HashMap<usize, (RectId, RectId)>>,
    input_context: String, // things may be displayed differently based on context
    rerow_flag: bool,

    rendered_buffer: Option<String>,


    rect_help_window: RectId,
    flag_context_changed: bool,

    rendered_viewport_y_offset: usize,

    rendered_viewport_size: Option<(usize, usize)>,
    rendered_viewport_offset: Option<usize>,
    rendered_formatter: Option<FormatterRef>,
    rendered_cursor: Option<(usize, usize)>

}

impl FrontEnd {
    pub fn new() -> FrontEnd {
        let mut rectmanager = RectManager::new();
        let rect_display_wrapper = rectmanager.new_rect(wrecked::ROOT).ok().unwrap();
        let id_display_bits = rectmanager.new_rect(rect_display_wrapper).ok().unwrap();
        let id_display_human = rectmanager.new_rect(rect_display_wrapper).ok().unwrap();
        let rect_meta = rectmanager.new_rect(wrecked::ROOT).ok().unwrap();
        let rect_feedback = rectmanager.new_rect(rect_meta).ok().unwrap();
        let rect_offset = rectmanager.new_rect(rect_meta).ok().unwrap();
        let rect_scrollbar = rectmanager.new_rect(rect_display_wrapper).ok().unwrap();
        let rect_help_window = rectmanager.new_rect(wrecked::ROOT).ok().unwrap();
        rectmanager.detach(rect_help_window);

        let mut frontend = FrontEnd {
            rectmanager,
            active_row_map: HashMap::new(),
            cells_to_refresh: HashSet::new(),
            rows_to_refresh: HashSet::new(),
            active_cursor_cells: HashSet::new(),

            rect_display_wrapper,
            rect_meta,
            rect_feedback,
            rect_offset,
            rect_scrollbar,
            rect_help_window,
            rects_display: (id_display_bits, id_display_human),
            row_dict: HashMap::new(),
            cell_dict: HashMap::new(),
            rendered_viewport_y_offset: 9999,

            input_context: "DEFAULT".to_string(),
            rerow_flag: false,
            rendered_buffer: None,

            flag_context_changed: false,
            rendered_viewport_size: None,
            rendered_viewport_offset: None,
            rendered_formatter: None,
            rendered_cursor: None
        };


        frontend
    }

    pub fn force_rerow(&mut self) {
        self.rerow_flag = true;
    }

    pub fn set_input_context(&mut self, new_context: &str) {
        self.flag_context_changed = true;
        self.input_context = new_context.to_string();
    }

    pub fn tick(&mut self, shell: &mut Shell) -> Result<(), Box::<dyn Error>> {
        let editor = shell.get_editor_mut();
        if !editor.is_loading() {
            let new_viewport_size = editor.get_viewport_size();
            let new_viewport_offset = editor.get_viewport_offset();
            let changed_viewport_size = Some(new_viewport_size) != self.rendered_viewport_size;
            let changed_viewport_offset = Some(new_viewport_offset) != self.rendered_viewport_offset;

            let new_cursor = (editor.get_cursor_offset(), editor.get_cursor_length());
            let changed_cursor = Some(new_cursor) != self.rendered_cursor;


            if changed_viewport_size {
                match self.setup_displays(editor) {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::SetupFailed(error))?
                    }
                }
            }

            if changed_viewport_size || changed_viewport_offset {
                match self.remap_active_rows(editor) {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::RemapFailed(error))?
                    }
                }
                self.rendered_viewport_size = Some(new_viewport_size);
                self.rendered_viewport_offset = Some(new_viewport_offset);
            }

            let changed_offsets = editor.fetch_changed_offsets();
            if !changed_offsets.is_empty() {
                let (viewport_width, viewport_height) = editor.get_viewport_size();
                let viewport_bottom = (editor.get_viewport_offset() / viewport_width) + viewport_height;
                for (i, rippled) in changed_offsets.iter() {
                    let start_row = i / viewport_width;
                    if *rippled && viewport_bottom >= start_row {
                        for y in start_row .. viewport_bottom + 1 {
                            self.rows_to_refresh.insert(y);
                        }
                    } else {
                        self.rows_to_refresh.insert(start_row);
                    }
                }
            }

            if !self.rows_to_refresh.is_empty() {
                let tmp_rows_to_refresh: Vec<usize> = self.rows_to_refresh.drain().collect();
                for y in tmp_rows_to_refresh.iter() {
                    match self.set_row_characters(editor, *y) {
                        Ok(_) => {}
                        Err(error) => {
                            Err(SbyteError::RowSetFailed(error))?
                        }
                    }
                }
            }


            if changed_cursor || changed_viewport_size || changed_viewport_offset {
                match self.apply_cursor(editor) {
                    Ok(_) => {}
                    Err(error) => {
                        Err(SbyteError::ApplyCursorFailed(error))?
                    }
                }
            }

            if changed_cursor {
                self.display_user_offset(editor)?;
                self.rendered_cursor = Some(new_cursor);
            }


            let mut feedback_or_error = false;
            match shell.fetch_feedback() {
                Some(msg) => {
                    self.display_user_message(msg.clone())?;
                    feedback_or_error = true;
                }
                None => { }
            }

            match shell.fetch_error() {
                Some(msg) => {
                    self.display_user_error(msg.clone())?;
                    feedback_or_error = true;
                }
                None => { }
            }


            if !feedback_or_error && (changed_cursor || self.flag_context_changed) {
                self.clear_feedback()?;
            }

            self.display_command_line(shell);


            match self.rectmanager.render() {
                Ok(_) => {}
                Err(error) => {
                    Err(SbyteError::DrawFailed(error))?;
                }
            }

            self.flag_context_changed = false;
        }

        Ok(())
    }

    pub fn auto_resize(&mut self, shell: &mut Shell) -> bool {
        let editor = shell.get_editor_mut();
        let new_formatter = editor.get_active_formatter_ref();
        if self.rectmanager.auto_resize() || Some(new_formatter) != self.rendered_formatter {
            let delay = time::Duration::from_nanos(1_000);
            thread::sleep(delay);

            let viewport_height = self.get_viewport_height();
            let screensize = self.size();
            let display_ratio = editor.get_display_ratio() as f64;
            let r: f64 = 1f64 / display_ratio;
            let a: f64 = 1f64 - (1f64 / (r + 1f64));
            let base_width = ((screensize.0 as f64 - 1f64) * a) as usize;

            let cursor_offset = editor.get_cursor_real_offset();
            let cursor_length = editor.get_cursor_real_length();
            editor.set_viewport_offset(0);
            editor.set_cursor_length(1);
            editor.set_cursor_offset(0);

            editor.set_viewport_size(base_width, viewport_height);
            editor.set_cursor_offset(cursor_offset);
            editor.set_cursor_length(cursor_length);
            self.rendered_formatter = Some(new_formatter);
            true
        } else {
            false
        }
    }

    fn remap_active_rows(&mut self, editor: &Editor) -> Result<(), WreckedError> {
        let (width, height) = editor.get_viewport_size();

        let initial_y = self.rendered_viewport_y_offset as isize;
        let new_y = (editor.get_viewport_offset() / width) as isize;
        self.rendered_viewport_y_offset = new_y as usize;

        let diff: usize;
        if new_y > initial_y {
            diff = (new_y - initial_y) as usize;
        } else {
            diff = (initial_y - new_y) as usize;
        }

        let force_rerow = self.rerow_flag;
        self.rerow_flag = false;

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
                    self.rows_to_refresh.insert(*y + (new_y as usize));
                }
            }
        }

        Ok(())
    }

    fn setup_displays(&mut self, editor: &Editor) -> Result<(), WreckedError> {
        // Assumes that the viewport size AND the rectmanager size are correctly set at this point
        let full_width = self.rectmanager.get_width();
        let full_height = self.rectmanager.get_height();
        let (viewport_width, viewport_height) = editor.get_viewport_size();

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
        self.arrange_displays(editor)?;


        self.cell_dict.drain();
        self.row_dict.drain();

        let display_ratio = editor.get_display_ratio() as usize;
        let width_bits;
        if display_ratio != 1 {
            width_bits = max(1, display_ratio - 1);
        } else {
            width_bits = display_ratio;
        }


        let mut _bits_row_id: RectId;
        let mut _bits_cell_id: RectId;
        let mut _human_row_id: RectId;
        let mut _human_cell_id: RectId;
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

        self.active_cursor_cells.drain();
        self.force_rerow();

        Ok(())
    }

    fn arrange_displays(&mut self, editor: &Editor) -> Result<(), WreckedError> {
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

        let display_ratio = editor.get_display_ratio();
        let (vwidth, _vheight) = editor.get_viewport_size();

        let (bits_id, human_id) = self.rects_display;
        let human_display_width = vwidth;
        let bits_display_width = vwidth * display_ratio as usize;
        let remaining_space = full_width - bits_display_width - human_display_width;

        let bits_display_x = remaining_space / 2;
        let human_display_x = (remaining_space / 2) + bits_display_width;

        self.rectmanager.resize(bits_id, bits_display_width, display_height)?;
        self.rectmanager.set_position(bits_id, bits_display_x as isize, 0)?;

        self.rectmanager.resize(human_id, human_display_width, display_height)?;
        self.rectmanager.set_position(human_id, human_display_x as isize, 0)?;

        self.rectmanager.set_fg_color(self.rect_scrollbar, wrecked::Color::BRIGHTBLACK);
        self.rectmanager.resize(self.rect_scrollbar, 1, display_height);
        self.rectmanager.set_position(self.rect_scrollbar, (human_display_x + human_display_width) as isize, 0);
        for y in 0 .. display_height as isize {
            self.rectmanager.set_character(self.rect_scrollbar, 0, y, '\u{250B}');
        }

        Ok(())
    }

    fn set_row_characters(&mut self, editor: &Editor, absolute_y: usize) -> Result<(), WreckedError> {
        let human_formatter = OneToOneFormatter {};
        let active_formatter = editor.get_active_formatter();
        let (width, _height) = editor.get_viewport_size();
        let offset = width * absolute_y;

        let chunk = editor.get_chunk(offset, width);
        let relative_y = absolute_y - (editor.get_viewport_offset() / width);

        match self.cell_dict.get_mut(&relative_y) {
            Some(cellhash) => {
                for (_x, (rect_id_bits, rect_id_human)) in cellhash.iter_mut() {
                    self.rectmanager.clear_characters(*rect_id_human)?;
                    self.rectmanager.clear_children(*rect_id_bits)?;
                    self.rectmanager.clear_characters(*rect_id_bits)?;
                }

                let mut tmp_bits_str;
                let mut tmp_human_str;
                for (x, byte) in chunk.iter().enumerate() {
                    match cellhash.get(&x) {
                        Some((bits, human)) => {
                            match active_formatter.read_in(*byte) {
                                tmp_bits => {
                                    tmp_bits_str = match std::str::from_utf8(tmp_bits.as_slice()) {
                                        Ok(valid) => {
                                            valid
                                        }
                                        Err(_) => {
                                            // Shouldn't Happen
                                            "."
                                        }
                                    };
                                    for (i, c) in tmp_bits_str.chars().enumerate() {
                                        self.rectmanager.set_character(*bits, i as isize, 0, c)?;
                                    }
                                }
                            }
                            match human_formatter.read_in(*byte) {
                                (tmp_human, fmt_response) => {
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
                                }
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

    pub fn display_user_offset(&mut self, editor: &Editor) -> Result<(), WreckedError> {
        let mut cursor_string = format!("{}", editor.get_cursor_offset());
        let active_content = editor.get_active_content();
        let (viewport_width, viewport_height) = editor.get_viewport_size();

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

        let cursor_len = editor.get_cursor_length();
        let offset_display;
        if cursor_len == 1 {
            offset_display = format!("Offset: {} / {}", cursor_string, denominator)
        } else {
            offset_display = format!("Offset: {} ({}) / {}", cursor_string, cursor_len, denominator)
        };

        // Do Scrollbar
        if denominator > (viewport_width * viewport_height) {
            self.rectmanager.enable(self.rect_scrollbar);
            self.rectmanager.clear_children(self.rect_scrollbar);
            let handle = self.rectmanager.new_rect(self.rect_scrollbar).ok().unwrap();
            let scrollbar_height = self.rectmanager.get_rect_height(self.rect_scrollbar);

            let handle_height = max(1, (viewport_height * scrollbar_height) / (denominator / viewport_width));
            self.rectmanager.set_bg_color(handle, wrecked::Color::BRIGHTBLACK);
            self.rectmanager.set_fg_color(handle, wrecked::Color::BLACK);
            self.rectmanager.resize(handle, 1, handle_height);

            let handle_y = (editor.get_viewport_offset() * (scrollbar_height - handle_height)) / (denominator - (viewport_width * viewport_height));
            self.rectmanager.set_position(handle, 0, handle_y as isize);
        } else {
            self.rectmanager.disable(self.rect_scrollbar);
        }
        ///////////////

        let meta_width = self.rectmanager.get_rect_width(self.rect_meta);
        let x = meta_width - offset_display.len();
        self.rectmanager.resize(self.rect_offset, offset_display.len(), 1)?;
        self.rectmanager.resize(self.rect_feedback, meta_width - offset_display.len(), 1)?;

        self.rectmanager.set_position(self.rect_offset, x as isize, 0)?;
        self.rectmanager.set_string(self.rect_offset, 0, 0, &offset_display)?;

        Ok(())
    }

    pub fn display_user_message(&mut self, msg: String) -> Result<(), WreckedError> {
        self.clear_feedback()?;

        self.rectmanager.set_string(self.rect_feedback, 0, 0, &msg)?;
        self.rectmanager.set_bold_flag(self.rect_feedback)?;
        self.rectmanager.set_fg_color(self.rect_feedback, Color::BRIGHTCYAN)?;

        Ok(())
    }

    pub fn display_user_error(&mut self, msg: String) -> Result<(), WreckedError> {
        self.clear_feedback()?;
        self.rectmanager.set_string(self.rect_feedback, 0, 0, &msg)?;
        self.rectmanager.set_fg_color(self.rect_feedback, Color::RED)?;

        Ok(())
    }

    fn clear_feedback(&mut self) -> Result<(), WreckedError> {
        self.rectmanager.clear_characters(self.rect_feedback)?;
        self.rectmanager.clear_children(self.rect_feedback)?;
        self.rectmanager.unset_bold_flag(self.rect_feedback)?;
        self.rectmanager.unset_fg_color(self.rect_feedback)?;

        Ok(())
    }

    pub fn _unapply_cursor(&mut self, editor: &Editor) -> Result<(), WreckedError> {
        // (They may no longer exist, but that's ok)
        for (bits, human) in self.active_cursor_cells.drain() {
            self.rectmanager.clear_children(bits);
            self.rectmanager.unset_invert_flag(bits).ok();
            self.rectmanager.unset_invert_flag(human).ok();
        }

        Ok(())
    }

    pub fn apply_cursor(&mut self, editor: &Editor) -> Result<(), WreckedError> {
        let (viewport_width, viewport_height) = editor.get_viewport_size();
        let viewport_offset = editor.get_viewport_offset();
        let cursor_offset = editor.get_cursor_offset();
        let cursor_length = editor.get_cursor_length();

        // First clear previously applied
        self._unapply_cursor(editor);

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
                            self.rectmanager.clear_children(*bits);
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

        match self.input_context.as_str() {
            "OVERWRITE_ASCII" | "OVERWRITE_HEX" | "OVERWRITE_BIN" | "OVERWRITE_DEC" => {
                let subcursor_length = editor.get_subcursor_length();
                let subcursor_offset = editor.get_subcursor_offset();
                let suboffset_cell = cursor_offset + (subcursor_offset / subcursor_length);

                y = (suboffset_cell - viewport_offset) / viewport_width;
                x = (suboffset_cell - viewport_offset) % viewport_width;
                match self.cell_dict.get(&y) {
                    Some(cellhash) => {
                        match cellhash.get(&x) {
                            Some((bits, human)) => {
                                match self.rectmanager.new_rect(*bits) {
                                    Ok(digit_cell) => {
                                        let digit_pos = (subcursor_offset % subcursor_length) as isize;
                                        let c = match self.rectmanager.get_character(*bits, digit_pos, 0) {
                                            Ok(_c) => { _c }
                                            Err(e) => { 'X' }
                                        };

                                        self.rectmanager.set_character(digit_cell, 0, 0, c);
                                        self.rectmanager.set_position(digit_cell, digit_pos, 0);
                                        self.rectmanager.set_invert_flag(digit_cell);
                                        self.rectmanager.set_underline_flag(digit_cell);
                                    }
                                    Err(_) => ()
                                }
                            }
                            None => ()
                        }
                    }
                    None => ()
                }
            }
            _ => {}
        }


        Ok(())
    }

    pub fn display_command_line(&mut self, shell: &Shell) -> Result<(), WreckedError> {
        if self.rendered_buffer != shell.buffer_get() {
            match shell.buffer_get() {
                Some(buffer) => {
                    self.clear_feedback()?;

                    let cursor_x = buffer.len() + 1;
                    let cursor_id = self.rectmanager.new_rect(self.rect_feedback).ok().unwrap();

                    self.rectmanager.resize(cursor_id, 1, 1)?;
                    self.rectmanager.set_position(cursor_id, cursor_x as isize, 0)?;
                    self.rectmanager.set_invert_flag(cursor_id)?;

                    self.rectmanager.set_string(self.rect_feedback, 0, 0, &vec![":", &buffer].join(""))?;
                }
                None => { }
            }
            self.rendered_buffer = shell.buffer_get();
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

