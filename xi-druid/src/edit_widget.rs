use std::sync::Arc;

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget,
};

use druid::piet::{
    Color, FontFamily, PietText, PietTextLayout, RenderContext, Text, TextLayout,
    TextLayoutBuilder,
};

use druid::kurbo::{Line, Point, Vec2};

use xi_rope::Rope;

use xi_text_core::{EditOp, Measurement, SelRegion, Selection};

use crate::key_bindings::KeyBindings;
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
    // One per line
    layouts: Vec<Layout>,
}

struct Layout {
    piet_layout: PietTextLayout,
    cursors: Vec<Line>,
}

struct XiMeasurement<'a> {
    layouts: &'a [Layout],
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
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &XiState, data: &XiState, _env: &Env) {
        let mut text = ctx.text();
        self.update_layouts(data, &mut text);
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
        for layout in &self.layouts {
            ctx.draw_text(&layout.piet_layout, (x, y));
            for line in &layout.cursors {
                let xy = Vec2::new(x, y);
                // It should be possible to add Line + Vec2.
                let l2 = Line::new(line.p0 + xy, line.p1 + xy);
                ctx.stroke(l2, &Color::WHITE, 1.0);
            }
            y += 18.0;
        }
    }
}

impl EditWidget {
    fn update_layouts(&mut self, data: &XiState, factory: &mut PietText) {
        // In time, this will be more incremental.
        let font_family = FontFamily::MONOSPACE;

        self.layouts.clear();
        let mut offset = 0;
        let mut selections = &**data.sel;
        for l in data.text.lines_raw(..) {
            let mut end = l.len();
            if l.ends_with('\n') {
                end -= 1;
            }
            if l[..end].ends_with('\r') {
                end -= 1;
            }
            let trim = &l[..end];
            let piet_layout: druid::piet::PietTextLayout =
                factory.new_text_layout(&trim)
                .font(font_family.clone(), 14.0)
                .text_color(Color::WHITE)
                .build()
                .unwrap();

            let mut cursors = Vec::new();
            while let Some(sel_region) = selections.first() {
                if sel_region.end <= offset + trim.len() {
                    let hit = piet_layout.hit_test_text_position(sel_region.end - offset);
                    let pt = hit.point;
                    let height = 18.0;
                    let line = Line::new(pt, pt + Vec2::new(0.0, height));
                    cursors.push(line);
                    selections = &selections[1..];
                } else {
                    break;
                }
            }
            let layout = Layout {
                piet_layout,
                cursors,
            };
            self.layouts.push(layout);
            offset += l.len();
        }
        // TODO: deal with empty last line
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
    fn n_visual_lines(&self, _line_num: usize) -> usize {
        1
    }

    fn to_pos(&self, line_num: usize, offset: usize) -> (f64, usize) {
        let layout = &self.layouts[line_num];
        let x = layout
            .piet_layout
            .hit_test_text_position(offset).point.x;
        (x, 0)
    }

    fn from_pos(&self, line_num: usize, horiz: f64, _visual_line: usize) -> usize {
        let layout = &self.layouts[line_num];
        let point = Point::new(horiz, 0.0);
        layout
            .piet_layout
            .hit_test_point(point)
            .idx
    }
}
