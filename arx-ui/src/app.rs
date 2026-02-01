use egui::Context;
use egui_async::EguiAsyncPlugin;
use jbk::EntryRange;
use libarx::FileEntry;
use std::path::PathBuf;

use crate::models::{AppModel, Model};
use crate::widgets::{Actionner, Breadcrumbs, FileList, MenuBar, Spinner, StatusBar, Widget};

pub enum Action {
    Enter((EntryRange, String)),
    Open(FileEntry),
    LoadArchive(PathBuf),
    JumpTo(usize),
    ExtractAll,
    ExtractOne(FileEntry),
    ExtractDir((EntryRange, String)),
}

impl Actionner for Option<Action> {
    type Action = Action;
    fn trigger(&mut self, action: Action) {
        *self = Some(action)
    }
}

#[derive(Default)]
pub struct ArxApp {
    model: AppModel,
}

impl ArxApp {
    pub fn new(cc: &eframe::CreationContext<'_>, archive: Option<String>) -> Self {
        cc.egui_ctx.all_styles_mut(|style| {
            style.interaction.selectable_labels = false;
        });
        Self {
            model: AppModel::new(archive),
        }
    }
}

impl eframe::App for ArxApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.plugin_or_default::<EguiAsyncPlugin>();
        let mut action = None;

        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            MenuBar::new(&self.model).interact(ui, &mut action)
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            StatusBar::new(&self.model).interact(ui, &mut action);
        });

        egui::TopBottomPanel::top("breadcrumbs").show(ctx, |ui| {
            if let Some(model) = &self.model.archive {
                Breadcrumbs::new(model).interact(ui, &mut action);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.model.background_task.is_pending() {
                Spinner.interact(ui, &mut action);
            }
            if let Some(archive) = &self.model.archive {
                FileList::new(archive).interact(ui, &mut action);
            }
        });

        if let Some(a) = action {
            self.model.update(a)
        }

        if let Some(message) = self.model.error_msg.take() {
            let ctx = ctx.clone();
            rfd::MessageDialog::new()
                .set_title("Error")
                .set_description(message)
                .set_level(rfd::MessageLevel::Error)
                .show();
            ctx.request_repaint();
        }
    }
}
