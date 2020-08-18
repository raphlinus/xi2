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

use crate::{SelRegion, Selection};

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
    pub fn update_region(&self, r: SelRegion, text: &Rope, modify: bool) -> SelRegion {
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
            _ => todo!(),
        };
        SelRegion::new(if modify { r.start } else { offset }, offset).with_horiz(horiz)
    }

    pub fn update_selection(&self, s: &Selection, text: &Rope, modify: bool) -> Selection {
        let mut result = Selection::new();
        for &r in s {
            let new_region = self.update_region(r, text, modify);
            result.add_region(new_region);
        }
        result
    }
}
