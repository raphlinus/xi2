
use kurbo::Rect;
use piet::{FillRule, FontBuilder, RenderContext, Text, TextLayoutBuilder};
use piet_common::Piet;

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::Widget;
use druid::{
    BoxConstraints, Geometry, HandlerCtx, Id, KeyEvent, KeyVariant, LayoutCtx, LayoutResult, PaintCtx, Ui,
    UiMain, UiState,
};

struct EditWidget {
    // TODO: change to xi-rope
    text: String,
}

impl EditWidget {
    fn get_layout(&self, rt: &mut Piet, font_size: f32) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = rt
            .text()
            .new_font_by_name("Consolas", font_size)
            .unwrap()
            .build()
            .unwrap();
        rt.text()
            .new_text_layout(&font, &self.text)
            .unwrap()
            .build()
            .unwrap()
    }
}

impl Widget for EditWidget {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size);
        let brush = paint_ctx.render_ctx.solid_brush(0xf0f0eaff).unwrap();

        let pos = (geom.pos.0, geom.pos.1 + font_size);
        paint_ctx.render_ctx.draw_text(&text_layout, pos, &brush);

    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((500.0, 400.0)))
    }

    fn key(&mut self, event: &KeyEvent, ctx: &mut HandlerCtx) -> bool {
        let dbg = match event.key {
            KeyVariant::Vkey(i) => format!("vkey {}", i),
            KeyVariant::Char(c) => format!("char {:?}", c),
        };
        println!("key {} {}", dbg, event.mods);
        match event.key {
            KeyVariant::Char(c) => {
                self.text.push(c);
                ctx.invalidate();
            }
            _ => (),
        }
        false
    }
}

impl EditWidget {
    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

fn build_ui(ui: &mut UiState) {
    let edit_widget = EditWidget {
        text: "".to_string(),
    }.ui(ui);
    let root = edit_widget;
    ui.set_root(root);
    ui.set_focus(Some(root));
}

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    build_ui(&mut state);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Xi2");
    let window = builder.build().expect("window building");
    window.show();
    run_loop.run();
}
