pub fn show(
    ui: &mut gasket_ui::egui::Ui,
    data: &crate::api::hello::Response,
    nav: &mut gasket_ui::Nav,
) {
    ui.heading("Hello");
    ui.label(&data.message);
    ui.add_space(8.0);
    ui.label("Name");
    ui.text_edit_singleline(nav.field("name"));
    if ui.button("Submit").clicked() {
        let name = nav.field("name").clone();
        nav.post(gasket_ui::serde_json::json!({ "name": name }));
    }
}
