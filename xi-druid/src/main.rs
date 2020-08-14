use druid::widget::Label;
use druid::{
    AppLauncher, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Size, UpdateCtx, Widget, WindowDesc,
};

use druid::piet::{Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayoutBuilder};

#[derive(Clone, Data)]
struct XiState {
    text: String,
}

#[derive(Default)]
struct EditWidget {
    // One per line
    layouts: Vec<PietTextLayout>,
}

impl Widget<XiState> for EditWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut XiState, env: &Env) {
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &XiState, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => self.update_layouts(&data.text, &mut ctx.text()),
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &XiState, data: &XiState, env: &Env) {
        let mut text = ctx.text();
        self.update_layouts(&data.text, &mut text);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &XiState,
        env: &Env,
    ) -> druid::Size {
        bc.constrain(Size::new(400.0, 400.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &XiState, env: &Env) {
        let mut y = 20.0;
        for layout in &self.layouts {
            ctx.draw_text(layout, (10.0, y), &Color::WHITE);
            y += 20.0;
        }
    }
}

impl EditWidget {
    fn update_layouts(&mut self, text: &str, factory: &mut PietText) {
        let font = factory.new_font_by_name("Segoe UI", 14.0).build().unwrap();
        let layout: druid::piet::PietTextLayout = factory.new_text_layout(&font, "This is the text", None).build().unwrap();
        self.layouts = vec![layout];

    }
}

pub fn main() {
    let main_window = WindowDesc::new(build_root_widget);
    let initial_state = XiState { text: "This is the text".into() };
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<XiState> {
    EditWidget::default()
}
