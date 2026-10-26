pub struct State;

impl State {
    pub fn new() -> Self {
        Self
    }

    pub fn show(
        &mut self,
        ui: &mut gasket_ui::egui::Ui,
        data: &crate::api::pulse::Response,
        _nav: &mut gasket_ui::Nav,
    ) {
        ui.heading("Pulse");
        ui.label(format!("count: {}", data.count));
        ui.label(&data.message);
    }
}
