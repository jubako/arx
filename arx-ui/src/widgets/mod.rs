mod breadcrumbs;
mod file_list;
mod menu_bar;
mod status_bar;
pub use breadcrumbs::Breadcrumbs;
pub use file_list::FileList;
pub use menu_bar::MenuBar;
pub use status_bar::StatusBar;

use egui::Ui;

pub trait Actionner {
    type Action;
    fn trigger(&mut self, action: Self::Action);
}

pub trait Widget {
    type Action;

    fn interact(&self, ui: &mut Ui) -> Option<Self::Action>;
}

pub trait View {
    type Action;

    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>);
}

pub struct Spinner;

impl Widget for Spinner {
    type Action = ();
    fn interact(&self, ui: &mut Ui) -> Option<()> {
        egui::Modal::new(egui::Id::new("Spinner")).show(ui.ctx(), |ui| ui.spinner());
        None
    }
}
