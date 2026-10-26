pub struct State;

impl State {
    pub fn new() -> Self {
        Self
    }

    pub fn show(
        &mut self,
        ui: &mut gasket_ui::egui::Ui,
        data: &crate::api::index::Response,
        nav: &mut gasket_ui::Nav,
    ) {
        ui.heading(&data.title);
        ui.label("This page only draws the route. The buttons are links in that JSON.");
        for link in &data.links {
            if ui.button(&link.title).clicked() {
                nav.open(&link.page);
            }
        }
        ui.add_space(8.0);
        ui.label(&data.note);
    }
}
