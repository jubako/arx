use super::{Actionner, Widget};
use crate::app::Action;
use crate::models::AppModel;
use egui::{global_theme_preference_switch, Layout, Ui};

pub struct StatusBar<'a> {
    status_message: &'a str,
    archive_file_name: Option<String>,
}

impl<'a> StatusBar<'a> {
    pub fn new(model: &'a AppModel) -> Self {
        Self {
            status_message: &model.status_message,
            archive_file_name: model
                .archive
                .as_ref()
                .and_then(|a| a.path.file_name())
                .map(|os| os.to_string_lossy().to_string()),
        }
    }
}

impl Widget for StatusBar<'_> {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, _actionner: &mut dyn Actionner<Action = Self::Action>) {
        ui.horizontal(|ui| {
            ui.label(self.status_message);

            if let Some(archive_file_name) = &self.archive_file_name {
                ui.separator();
                ui.label(format!("Archive: {archive_file_name}",));
            }
            ui.with_layout(
                Layout::right_to_left(egui::Align::Min),
                global_theme_preference_switch,
            );
        });
    }
}
