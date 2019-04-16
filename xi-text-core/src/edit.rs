// Copyright 2019 The xi-editor Authors.
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

use std::ops::Deref;

use xi_rope::{DeltaBuilder, Interval, Rope, RopeDelta};

use crate::backspace::offset_for_delete_backwards;
use crate::selection::Selection;

pub struct EditRequest {
    pub edit: Option<(RopeDelta, EditType)>,
    pub new_sel: Option<Selection>,
}

impl EditRequest {
    fn new_edit(delta: RopeDelta, edit_type: EditType) -> EditRequest {
        EditRequest {
            edit: Some((delta, edit_type)),
            new_sel: None,
        }
    }

    fn nop() -> EditRequest {
        EditRequest {
            edit: None,
            new_sel: None,
        }
    }
}

/// The type of the edit.
///
/// TODO: possibly remove some.
pub enum EditType {
    /// A catchall for edits that don't fit elsewhere, and which should
    /// always have their own undo groups; used for things like cut/copy/paste.
    Other,
    /// An insert from the keyboard/IME (not a paste or a yank).
    InsertChars,
    InsertNewline,
    /// An indentation adjustment.
    Indent,
    Delete,
    Undo,
    Redo,
    Transpose,
    Surround,
}

pub struct EditCtx<'a> {
    pub text: &'a Rope,
    pub sel: &'a Selection,
    // TODO: breaks and measurement
}

impl<'a> EditCtx<'a> {
    pub fn insert(&self, text: impl Into<Rope>) -> EditRequest {
        let rope = text.into();
        let mut builder = DeltaBuilder::new(self.text.len());
        for region in self.sel.deref() {
            let iv = Interval::new(region.min(), region.max());
            builder.replace(iv, rope.clone());
        }
        EditRequest::new_edit(builder.build(), EditType::InsertChars)
    }

    pub fn delete_backward(&self) -> EditRequest {
        let mut builder = DeltaBuilder::new(self.text.len());
        for region in self.sel.deref() {
            let start = offset_for_delete_backwards(&region, &self.text);
            let iv = Interval::new(start, region.max());
            if !iv.is_empty() {
                builder.delete(iv);
            }
        }

        if !builder.is_empty() {
            EditRequest::new_edit(builder.build(), EditType::Delete)
        } else {
            EditRequest::nop()
        }
    }
}
