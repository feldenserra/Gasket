use crate::api::about::{Note, Response};

pub struct State {
    text: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut gasket_ui::egui::Ui, data: &Response, nav: &mut gasket_ui::Nav) {
        ui.heading(&data.title);
        ui.label(&data.body);
        ui.add_space(8.0);
        ui.label("Note");
        ui.text_edit_singleline(&mut self.text);
        if ui.button("Submit").clicked() {
            nav.submit(&Note {
                text: self.text.clone(),
            });
        }
    }
}
