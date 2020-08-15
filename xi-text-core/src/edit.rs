// Copyright 2020 The xi-editor Authors.
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

//! Edit operations.

use xi_rope::{DeltaBuilder, Rope, RopeDelta};

use crate::backspace;
use crate::selection::{InsertDrift, Selection};

/// An edit operation.
///
/// We explicitly represent an edit operation.
pub enum EditOp {
    Insert(String),
    Backspace,
}

impl EditOp {
    // Maybe return `Option<Selection>`? There's a chance it might not change.
    // Also: needs measurement.
    pub fn apply(&self, text: &mut Rope, sel: &Selection) -> Selection {
        match self {
            EditOp::Insert(s) => {
                let rope = Rope::from(s);
                let mut builder = DeltaBuilder::new(text.len());
                for region in sel {
                    builder.replace(region.min()..region.max(), rope.clone());
                }
                apply_delta(text, sel, &builder.build())
            }
            EditOp::Backspace => {
                let mut builder = DeltaBuilder::new(text.len());
                for region in sel {
                    let start = backspace::offset_for_delete_backwards(region, text);
                    if start != region.max() {
                        builder.delete(start..region.max());
                    }
                }
                apply_delta(text, sel, &builder.build())
            }
        }
    }
}

fn apply_delta(text: &mut Rope, sel: &Selection, delta: &RopeDelta) -> Selection {
    *text = delta.apply(&text);
    sel.apply_delta(delta, true, InsertDrift::Default)
}
