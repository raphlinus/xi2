use druid::{
    AppLauncher, BoxConstraints, Data, Env, Event, EventCtx, KbKey, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, Size, UpdateCtx, Widget, WindowDesc,
};

use druid::piet::{
    Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayoutBuilder,
};

use xi_rope::Rope;

mod util;

#[derive(Clone, Data)]
struct XiState {
    #[data(same_fn = "util::rope_eq")]
    text: Rope,
}

#[derive(Default)]
struct EditWidget {
    // One per line
    layouts: Vec<PietTextLayout>,
}

impl Widget<XiState> for EditWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut XiState, _env: &Env) {
        match event {
            Event::KeyDown(k) => match &k.key {
                KbKey::Character(c) => {
                    let len = data.text.len();
                    data.text.edit(len..len, c);
                }
                KbKey::Enter => {
                    let len = data.text.len();
                    data.text.edit(len..len, "\n");
                }
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
}

pub fn main() {
    let main_window = WindowDesc::new(build_root_widget);
    let initial_state = XiState {
        text: "This is the text".into(),
    };
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<XiState> {
    EditWidget::default()
}
