mod edit_widget;
mod util;

use druid::{AppLauncher, Widget, WindowDesc};

use edit_widget::{EditWidget, XiState};

pub fn main() {
    let main_window = WindowDesc::new(build_root_widget);
    let initial_state = XiState::new("This is the text");
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<XiState> {
    EditWidget::default()
}
