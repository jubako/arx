use super::{Actionner, Widget};
use crate::app::Action;
use crate::models::AppModel;
use egui::Ui;

pub struct MenuBar {
    has_archive: bool,
}

impl MenuBar {
    pub fn new(model: &AppModel) -> Self {
        Self {
            has_archive: model.has_archive(),
        }
    }
}

impl Widget for MenuBar {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>) {
        egui::MenuBar::new().ui(ui, |ui| {
            if ui.button("📂 Open Archive").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Arx Archive", &["arx"])
                    .pick_file()
                {
                    actionner.trigger(Action::LoadArchive(file));
                }
            }

            ui.add_enabled_ui(self.has_archive, |ui| {
                if ui.button("📦 Extract All").clicked() {
                    actionner.trigger(Action::ExtractAll);
                }
            });
        });
    }
}
