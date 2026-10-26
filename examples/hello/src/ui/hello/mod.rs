pub fn show(
    ui: &mut gasket_ui::egui::Ui,
    data: &crate::api::hello::Response,
    _nav: &mut gasket_ui::Nav,
) {
    ui.heading("Hello");
    ui.label("One HTTP GET. Refresh asks /api/hello again.");
    ui.label(&data.message);
    ui.label(&data.status);
}
