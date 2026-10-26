pub struct State {
    name: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            name: String::new(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut gasket_ui::egui::Ui,
        data: &crate::api::hello::Response,
        nav: &mut gasket_ui::Nav,
    ) {
        ui.heading("Hello");
        ui.label(&data.message);
        ui.add_space(8.0);
        ui.label("Name");
        ui.text_edit_singleline(&mut self.name);
        if ui.button("Submit").clicked() {
            let name = self.name.clone();
            nav.post(gasket_ui::serde_json::json!({ "name": name }));
        }
    }
}
