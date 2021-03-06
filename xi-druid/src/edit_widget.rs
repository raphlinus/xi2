use std::sync::Arc;

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget,
};

use druid::piet::{
    Color, FontFamily, PietText, RenderContext, Text, TextLayout, TextLayoutBuilder,
};

use druid::kurbo::{Line, Point, Vec2};

use xi_rope::Rope;

use xi_text_core::{EditOp, Measurement, SelRegion, Selection};

use crate::key_bindings::KeyBindings;
use crate::layout_rope::{Layout, LayoutRope, LayoutRopeBuilder};
use crate::util;

#[derive(Clone, Data)]
pub struct XiState {
    #[data(same_fn = "util::rope_eq")]
    text: Rope,
    sel: Arc<Selection>,
}

#[derive(Default)]
pub struct EditWidget {
    bindings: KeyBindings,
    layouts: LayoutRope,
    // Each cursor is represented as the paragraph number and a line
    // relative to the start of that paragraph.
    cursors: Vec<(usize, Line)>,
}

struct XiMeasurement<'a> {
    layouts: &'a LayoutRope,
}

impl Widget<XiState> for EditWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut XiState, _env: &Env) {
        match event {
            Event::KeyDown(k) => {
                if let Some(op) = self.bindings.map_key(k) {
                    self.apply_edit_op(data, op);
                }
            }
            Event::MouseDown(_) => {
                // TODO: request focus on startup; why isn't it a method on LifeCycleCtx?
                ctx.request_focus();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &XiState, _env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                self.update_layouts(data, &mut ctx.text());
                self.update_cursors(data);
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &XiState, data: &XiState, _env: &Env) {
        let mut text = ctx.text();
        self.update_layouts(data, &mut text);
        self.update_cursors(data);
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &XiState,
        _env: &Env,
    ) -> druid::Size {
        // TODO: should do layout and measure height.
        bc.constrain(Size::new(400.0, 400.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &XiState, _env: &Env) {
        let x = 10.0;
        let mut y = 12.0;
        let mut para_ix = 0;
        let mut cursor_ix = 0;
        for (height, layout) in &self.layouts {
            ctx.draw_text(layout.piet_layout(), (x, y));
            while let Some((c_para, line)) = self.cursors.get(cursor_ix) {
                if para_ix != *c_para {
                    break;
                }
                let xy = Vec2::new(x, y);
                // It should be possible to add Line + Vec2.
                let l2 = Line::new(line.p0 + xy, line.p1 + xy);
                ctx.stroke(l2, &Color::WHITE, 1.0);
                cursor_ix += 1;
            }
            y += height.to_f64();
            para_ix += 1;
        }
    }
}

impl EditWidget {
    fn update_layouts(&mut self, data: &XiState, factory: &mut PietText) {
        // In time, this will be more incremental.
        let font_family = FontFamily::MONOSPACE;

        let mut builder = LayoutRopeBuilder::new();
        let mut offset = 0;
        let mut selections = &**data.sel;
        let mut text = data.text.clone();
        // This is an expedient hack to make sure we get a layout and can draw
        // the cursor for the last (empty) line, if it exists.
        if text.is_empty() || text.byte_at(text.len() - 1) == b'\n' {
            text = text + "\n".into();
        }
        for l in text.lines_raw(..) {
            let mut end = l.len();
            if l.ends_with('\n') {
                end -= 1;
            }
            if l[..end].ends_with('\r') {
                end -= 1;
            }
            let trim = &l[..end];
            let piet_layout: druid::piet::PietTextLayout = factory
                .new_text_layout(&trim)
                .max_width(400.0)
                .font(font_family.clone(), 14.0)
                .text_color(Color::WHITE)
                .build()
                .unwrap();

            let mut cursors = Vec::new();
            while let Some(sel_region) = selections.first() {
                if sel_region.end <= offset + trim.len() {
                    let hit = piet_layout.hit_test_text_position(sel_region.end - offset);
                    // TODO: use line metrics, but good enough for a quick hack.
                    let pt = hit.point - Vec2::new(0.0, 12.0);
                    let height = 18.0;
                    let line = Line::new(pt, pt + Vec2::new(0.0, height));
                    cursors.push(line);
                    selections = &selections[1..];
                } else {
                    break;
                }
            }
            let layout = Layout::new(piet_layout);
            builder.push_layout(layout);
            offset += l.len();
        }
        self.layouts = builder.build()
    }

    fn update_cursors(&mut self, data: &XiState) {
        self.cursors.clear();
        for sel_region in &*data.sel {
            let cursor_offset = sel_region.end;
            let para_ix = data.text.line_of_offset(cursor_offset);
            let para_start = data.text.offset_of_line(para_ix);
            let piet_layout = self.layouts.get(para_ix).unwrap().1.piet_layout();
            let hit = piet_layout.hit_test_text_position(cursor_offset - para_start);
            // TODO: use line metrics, but good enough for a quick hack.
            let pt = hit.point - Vec2::new(0.0, 12.0);
            let height = 18.0;
            let line = Line::new(pt, pt + Vec2::new(0.0, height));
            self.cursors.push((para_ix, line));
        }
    }

    fn apply_edit_op(&mut self, data: &mut XiState, op: EditOp) {
        let measurement = self.measurement();
        let new_sel = op.apply(&mut data.text, &data.sel, &measurement);
        data.sel = Arc::new(new_sel);
    }

    fn measurement(&self) -> XiMeasurement {
        XiMeasurement {
            layouts: &self.layouts,
        }
    }
}

impl XiState {
    pub fn new(initial_text: impl Into<Rope>) -> XiState {
        let text = initial_text.into();
        let len = text.len();
        let sel = Selection::new_simple(SelRegion::new(len, len));
        XiState {
            text,
            sel: Arc::new(sel),
        }
    }
}

impl<'a> Measurement for XiMeasurement<'a> {
    fn n_visual_lines(&self, line_num: usize) -> usize {
        let layout = self.layouts.get(line_num).unwrap().1.piet_layout();
        layout.line_count()
    }

    fn to_pos(&self, line_num: usize, offset: usize) -> (f64, usize) {
        let layout = self.layouts.get(line_num).unwrap().1.piet_layout();
        let hit = layout.hit_test_text_position(offset);
        (hit.point.x, hit.line)
    }

    fn from_pos(&self, line_num: usize, horiz: f64, visual_line: usize) -> usize {
        let layout = self.layouts.get(line_num).unwrap().1.piet_layout();
        if let Some(metric) = layout.line_metric(visual_line) {
            let y = metric.y_offset + 0.5 * metric.height;
            let point = Point::new(horiz, y);
            layout.hit_test_point(point).idx
        } else {
            // This shouldn't happen, but provide a reasonable value.
            0
        }
    }
}
