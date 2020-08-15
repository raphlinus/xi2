use std::sync::Arc;

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, KbKey, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Size, UpdateCtx, Widget,
};

use druid::piet::{
    Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayoutBuilder,
};

use xi_rope::Rope;

use xi_text_core::{EditOp, SelRegion, Selection};

use crate::util;

#[derive(Clone, Data)]
pub struct XiState {
    #[data(same_fn = "util::rope_eq")]
    text: Rope,
    sel: Arc<Selection>,
}

#[derive(Default)]
pub struct EditWidget {
    // One per line
    layouts: Vec<PietTextLayout>,
}

impl Widget<XiState> for EditWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut XiState, _env: &Env) {
        match event {
            Event::KeyDown(k) => match &k.key {
                KbKey::Character(c) => {
                    // TODO: make this logic more sophisticated
                    if !k.mods.ctrl() {
                        self.apply_edit_op(data, EditOp::Insert(c.clone()));
                    }
                }
                KbKey::Enter => self.apply_edit_op(data, EditOp::Insert("\n".into())),
                KbKey::Backspace => self.apply_edit_op(data, EditOp::Backspace),
                _ => (),
            },
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
                self.update_layouts(&data.text, &mut ctx.text());
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &XiState, data: &XiState, _env: &Env) {
        let mut text = ctx.text();
        self.update_layouts(&data.text, &mut text);
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
        let mut y = 20.0;
        for layout in &self.layouts {
            ctx.draw_text(layout, (10.0, y), &Color::WHITE);
            y += 18.0;
        }
    }
}

impl EditWidget {
    fn update_layouts(&mut self, text: &Rope, factory: &mut PietText) {
        // In time, this will be more incremental.
        let text = text.to_string();
        let font = factory.new_font_by_name("Segoe UI", 14.0).build().unwrap();
        let layout: druid::piet::PietTextLayout =
            factory.new_text_layout(&font, &text, None).build().unwrap();
        self.layouts = vec![layout];

        self.layouts.clear();
        for l in text.lines() {
            let layout: druid::piet::PietTextLayout =
                factory.new_text_layout(&font, l, None).build().unwrap();
            self.layouts.push(layout);
        }
    }

    fn apply_edit_op(&mut self, data: &mut XiState, op: EditOp) {
        let new_sel = op.apply(&mut data.text, &data.sel);
        data.sel = Arc::new(new_sel);
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
