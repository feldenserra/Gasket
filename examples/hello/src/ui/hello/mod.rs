use crate::api::hello::{Response, Wish};

pub struct State {
    name: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            name: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut gasket_ui::egui::Ui, data: &Response, nav: &mut gasket_ui::Nav) {
        ui.heading("Hello");
        ui.label(&data.message);
        ui.add_space(8.0);
        ui.label("Name");
        ui.text_edit_singleline(&mut self.name);
        if ui.button("Submit").clicked() {
            nav.submit(&Wish {
                name: self.name.clone(),
            });
        }
        ui.label("Refresh reset state on this page");
        if ui.button("Refresh").clicked() {
            self.reset();
            nav.refresh();
        }
    }

    fn reset(&mut self) {
        self.name = String::new();
    }
}
