use super::{Actionner, Widget};
use crate::app::Action;
use crate::models::ArxModel;
use egui::{Layout, Popup, Sense, Ui};
use egui_extras::{Column, TableBuilder};
use jbk::EntryRange;
use libarx::{CommonEntry, FileEntry, FullEntry};

struct FileContextMenu<'a> {
    f: &'a FileEntry,
}

impl<'a> Widget for FileContextMenu<'a> {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>) {
        if ui.button("Open").clicked() {
            actionner.trigger(Action::Open(self.f.clone()))
        }
        ui.separator();
        if ui.button("Extract").clicked() {
            actionner.trigger(Action::ExtractOne(self.f.clone()));
        }
    }
}

struct DirContextMenu<'a> {
    range: EntryRange,
    path: &'a str,
}

impl<'a> DirContextMenu<'a> {
    fn new(range: EntryRange, path: &'a str) -> Self {
        Self { range, path }
    }
}

impl<'a> Widget for DirContextMenu<'a> {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>) {
        if ui.button("Enter").clicked() {
            actionner.trigger(Action::Enter((self.range, self.path.to_string())));
        }
        ui.separator();
        if ui.button("Extract").clicked() {
            actionner.trigger(Action::ExtractDir((self.range, self.path.to_string())))
        }
    }
}

pub struct FileList<'a> {
    entry_list: &'a [FullEntry],
}

impl<'a> FileList<'a> {
    pub fn new(model: &'a ArxModel) -> Self {
        Self {
            entry_list: &model.entry_list,
        }
    }
}

impl Widget for FileList<'_> {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>) {
        TableBuilder::new(ui)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(egui::Align::Min).with_main_wrap(false))
            .column(Column::auto().resizable(false))
            .column(Column::remainder())
            .column(Column::auto())
            .header(20., |mut header| {
                header.col(|_| {});
                header.col(|ui| {
                    ui.heading("Name");
                });
                header.col(|ui| {
                    ui.heading("Size");
                });
            })
            .body(|body| {
                body.rows(20., self.entry_list.len(), |mut row| {
                    let row_idx = row.index();
                    let entry = &self.entry_list[row_idx];
                    let path = String::from_utf8_lossy(entry.path()).to_string();
                    let (icon, size) = match entry {
                        libarx::Entry::File(f) => ("📄", Some(f.size())),
                        libarx::Entry::Link(_) => ("🔗", None),
                        libarx::Entry::Dir(_, _) => ("📁", None),
                    };
                    row.col(|ui| {
                        ui.label(icon);
                    });
                    row.col(|ui| {
                        ui.label(path.as_str());
                    });
                    row.col(|ui| {
                        ui.label(
                            size.map(|s| format_size(s.into_u64()))
                                .unwrap_or(String::new()),
                        );
                    });
                    let response = row.response();

                    Popup::context_menu(&response).show(|ui| match entry {
                        libarx::Entry::Dir(r, _) => {
                            DirContextMenu::new(*r, &path).interact(ui, actionner);
                        }
                        libarx::Entry::File(f) => {
                            FileContextMenu { f }.interact(ui, actionner);
                        }
                        _ => {}
                    });

                    if response.double_clicked() {
                        match entry {
                            libarx::Entry::Dir(r, _) => {
                                actionner.trigger(Action::Enter((*r, path)));
                            }
                            libarx::Entry::File(f) => {
                                actionner.trigger(Action::Open(f.clone()));
                            }
                            _ => {}
                        }
                    }
                });
            });
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
