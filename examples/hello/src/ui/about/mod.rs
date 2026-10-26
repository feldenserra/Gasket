pub fn show(
    ui: &mut gasket_ui::egui::Ui,
    data: &crate::api::about::Response,
    _nav: &mut gasket_ui::Nav,
) {
    ui.heading(&data.title);
    ui.label(&data.body);
}
