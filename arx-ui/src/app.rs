use anyhow::Result as AnyResult;
use egui::{global_theme_preference_switch, Context, Layout, Sense, TextBuffer, Ui};
use egui_extras::{Column, TableBuilder};
use libarx::{Arx, CommonEntry, ExtractBuilder};
use std::path::PathBuf;

#[derive(Default)]
pub struct AppModel {
    archive: Option<Arx>,
    path: PathBuf,
    roots: Vec<(jbk::EntryRange, String)>,
    status_message: String,
}

fn entries<'a>(
    archive: &Arx,
    root: Option<jbk::EntryRange>,
) -> AnyResult<impl ExactSizeIterator<Item = Result<libarx::FullEntry, libarx::BaseError>> + 'a> {
    let builder = libarx::RealBuilder::<libarx::FullBuilder>::new(&archive.properties);
    let read_entry = match root {
        None => {
            libarx::ReadEntry::new_owned(&archive.get_index_for_name("arx_root")?.unwrap(), builder)
        }
        Some(r) => libarx::ReadEntry::new_owned(&r, builder),
    };
    Ok(read_entry)
}

impl AppModel {
    fn new(path: Option<String>) -> AnyResult<Self> {
        let mut s: Self = Default::default();
        if let Some(path) = path {
            s.load_archive(path.into());
        }
        Ok(s)
    }

    fn load_archive(&mut self, path: PathBuf) {
        self.status_message = format!("Loading {}...", path.display());

        match Arx::new(&path) {
            Ok(archive) => {
                self.archive = Some(archive);
                self.path = path.clone();
                //self.apply_filter();
                self.status_message = format!(
                    "{} opened",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                self.roots.clear()
            }
            Err(e) => {
                self.status_message = format!("Error opening archive: {}", e);
            }
        }
    }
    fn extract_all(&self) {
        if let Some(archive) = &self.archive {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                std::fs::create_dir_all(&folder).unwrap();
                ExtractBuilder::new(&folder)
                    .overwrite(libarx::Overwrite::Skip)
                    .extract(archive, None)
                    .unwrap();
            }
        }
    }

    fn root(&self) -> Option<(jbk::EntryRange, String)> {
        self.roots.last().cloned()
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
            model: AppModel::new(archive).unwrap(),
        }
    }

    fn menubar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            if ui.button("📂 Open Archive").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Arx Archive", &["arx"])
                    .pick_file()
                {
                    self.model.load_archive(file);
                }
            }

            ui.add_enabled_ui(self.model.archive.is_some(), |ui| {
                if ui.button("📦 Extract All").clicked() {
                    self.model.extract_all();
                }
            });
        });
    }

    fn breadcrumbs(&mut self, ui: &mut Ui) {
        let mut to_split = None;
        ui.horizontal(|ui| {
            if ui.button("📂").clicked() {
                to_split = Some(0);
            }
            for (idx, dir) in self.model.roots.iter().enumerate() {
                ui.separator();
                if ui.button(&dir.1).clicked() {
                    to_split = Some(idx + 1);
                }
            }
        });
        to_split.map(|idx| self.model.roots.split_off(idx));
    }

    fn file_list(&mut self, ui: &mut Ui) {
        let root = self.model.root().map(|(r, _)| r);
        if let Some(archive) = self.model.archive.as_ref() {
            let mut entry_iter = entries(archive, root).unwrap().enumerate();
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
                    body.rows(20., entry_iter.len(), |mut row| {
                        let row_idx = row.index();
                        let mut skip_iter = entry_iter.by_ref().skip_while(|(i, _)| i < &row_idx);
                        let entry = skip_iter.next().unwrap().1;
                        let entry = entry.as_ref().unwrap();
                        let path = String::from_utf8_lossy(entry.path());
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

                        if response.double_clicked() {
                            if let libarx::Entry::Dir(r, _) = entry {
                                self.model.roots.push((*r, path.to_string()))
                            }
                        }
                    });
                });
        }
    }

    fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.model.status_message);

            if self.model.archive.is_some() {
                ui.separator();
                ui.label(format!(
                    "Archive: {}",
                    self.model
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
            }
            ui.with_layout(
                Layout::right_to_left(egui::Align::Min),
                global_theme_preference_switch,
            );
        });
    }
}

impl eframe::App for ArxApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            self.menubar(ui);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(5.0);
            self.status_bar(ui);
            ui.add_space(5.0);
        });

        egui::TopBottomPanel::top("breadcrumbs").show(ctx, |ui| {
            ui.add_space(5.0);
            self.breadcrumbs(ui);
            ui.add_space(5.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.file_list(ui);
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
