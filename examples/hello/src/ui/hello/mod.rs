pub fn show(ui: &mut gasket_ui::egui::Ui, data: &crate::api::hello::Response) {
    ui.heading("Hello");
    ui.label(&data.message);
}
