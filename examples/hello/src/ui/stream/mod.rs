pub fn show(
    ui: &mut gasket_ui::egui::Ui,
    data: &crate::api::stream::Response,
    _nav: &mut gasket_ui::Nav,
) {
    ui.heading("Stream");
    ui.label(format!("count: {}", data.count));
    ui.label(&data.message);
}
