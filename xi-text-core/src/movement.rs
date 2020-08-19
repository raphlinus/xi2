// Copyright 2017 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use xi_rope::Rope;

use crate::{Measurement, SelRegion, Selection};

/// The specification of a movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Movement {
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
    /// Move to the left by one word.
    LeftWord,
    /// Move to the right by one word.
    RightWord,
    /// Move to left end of visible line.
    LeftOfLine,
    /// Move to right end of visible line.
    RightOfLine,
    /// Move up one visible line.
    Up,
    /// Move down one visible line.
    Down,
    /// Move up one viewport height.
    UpPage,
    /// Move down one viewport height.
    DownPage,
    /// Move up to the next line that can preserve the cursor position.
    UpExactPosition,
    /// Move down to the next line that can preserve the cursor position.
    DownExactPosition,
    /// Move to the start of the text line.
    StartOfParagraph,
    /// Move to the end of the text line.
    EndOfParagraph,
    /// Move to the end of the text line, or next line if already at end.
    EndOfParagraphKill,
    /// Move to the start of the document.
    StartOfDocument,
    /// Move to the end of the document
    EndOfDocument,
}

impl Movement {
    /// Update a selection region by movement.
    // TODO: additional measurement stuff.
    pub fn update_region(
        &self,
        r: SelRegion,
        text: &Rope,
        measurement: &impl Measurement,
        modify: bool,
    ) -> SelRegion {
        let (offset, horiz) = match self {
            Movement::Left => {
                if r.is_caret() || modify {
                    if let Some(offset) = text.prev_grapheme_offset(r.end) {
                        (offset, None)
                    } else {
                        (0, r.horiz)
                    }
                } else {
                    (r.min(), None)
                }
            }
            Movement::Right => {
                if r.is_caret() || modify {
                    if let Some(offset) = text.next_grapheme_offset(r.end) {
                        (offset, None)
                    } else {
                        (r.end, r.horiz)
                    }
                } else {
                    (r.max(), None)
                }
            }
            Movement::Up => {
                let info = pos_info(&r, text, measurement, true, modify);
                if info.rel_line > 0 {
                    let rel_offset =
                        measurement.from_pos(info.line_num, info.horiz, info.rel_line - 1);
                    (info.line_start + rel_offset, Some(info.horiz))
                } else if info.line_num == 0 {
                    (0, Some(info.horiz))
                } else {
                    let prev_line = info.line_num - 1;
                    let n_lines = measurement.n_visual_lines(prev_line);
                    let prev_line_start = text.offset_of_line(prev_line);
                    let rel_offset = measurement.from_pos(prev_line, info.horiz, n_lines - 1);
                    (prev_line_start + rel_offset, Some(info.horiz))
                }
            }
            Movement::Down => {
                let info = pos_info(&r, text, measurement, false, modify);
                let n_lines = measurement.n_visual_lines(info.line_num);
                if info.rel_line + 1 < n_lines {
                    let rel_offset =
                        measurement.from_pos(info.line_num, info.horiz, info.rel_line + 1);
                    (info.line_start + rel_offset, Some(info.horiz))
                } else {
                    let next_line_start = text.offset_of_line(info.line_num + 1);
                    let offset = if next_line_start == text.len() {
                        next_line_start
                    } else {
                        let rel_offset = measurement.from_pos(info.line_num + 1, info.horiz, 0);
                        next_line_start + rel_offset
                    };
                    (offset, Some(info.horiz))
                }
            }
            _ => todo!(),
        };
        SelRegion::new(if modify { r.start } else { offset }, offset).with_horiz(horiz)
    }

    pub fn update_selection(
        &self,
        s: &Selection,
        text: &Rope,
        measurement: &impl Measurement,
        modify: bool,
    ) -> Selection {
        let mut result = Selection::new();
        for &r in s {
            let new_region = self.update_region(r, text, measurement, modify);
            result.add_region(new_region);
        }
        result
    }
}

struct PosInfo {
    line_num: usize,
    horiz: f64,
    line_start: usize,
    rel_line: usize,
}

fn pos_info(
    r: &SelRegion,
    text: &Rope,
    measurement: &impl Measurement,
    move_up: bool,
    modify: bool,
) -> PosInfo {
    let offset = if modify {
        r.end
    } else if move_up {
        r.min()
    } else {
        r.max()
    };
    let line_num = text.line_of_offset(offset);
    let line_start = text.offset_of_line(line_num);
    let rel_offset = offset - line_start;
    let (meas_horiz, rel_line) = measurement.to_pos(line_num, rel_offset);
    let horiz = r.horiz.unwrap_or(meas_horiz);
    PosInfo {
        line_num,
        horiz,
        line_start,
        rel_line,
    }
}
