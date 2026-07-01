use super::{Actionner, View};
use crate::app::Action;
use crate::models::ArxModel;
use egui::Ui;
use jbk::EntryRange;

pub struct Breadcrumbs<'a> {
    roots: &'a [(EntryRange, String)],
}

impl<'a> Breadcrumbs<'a> {
    pub fn new(model: &'a ArxModel) -> Self {
        Self {
            roots: &model.roots,
        }
    }
}

impl View for Breadcrumbs<'_> {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>) {
        ui.horizontal(|ui| {
            if ui.button("📂").clicked() {
                actionner.trigger(Action::JumpTo(0));
            }

            for (idx, dir) in self.roots.iter().enumerate() {
                ui.label("/");
                if ui.button(&dir.1).clicked() {
                    actionner.trigger(Action::JumpTo(idx + 1));
                }
            }
        });
    }
}
