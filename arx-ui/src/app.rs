use anyhow::Result as AnyResult;
use egui::{global_theme_preference_switch, Context, Layout, Sense, TextBuffer, Ui};
use egui_extras::{Column, TableBuilder};
use libarx::{Arx, ArxError, CommonEntry, ExtractBuilder};
use std::path::PathBuf;

#[derive(Default)]
struct ErrorMsg {
    error: Option<String>,
}

impl ErrorMsg {
    fn catch<T, E: ToString>(&mut self, result: Result<T, E>) -> Option<T> {
        if let Err(e) = &result {
            self.error = Some(e.to_string());
        }
        result.ok()
    }

    fn take(&mut self) -> Option<String> {
        self.error.take()
    }
}

struct ArxModel {
    archive: Arx,
    path: PathBuf,
    roots: Vec<(jbk::EntryRange, String)>,
    entry_list: Vec<libarx::FullEntry>,
}

impl ArxModel {
    pub fn open(path: PathBuf) -> Result<Self, libarx::ArxError> {
        let archive = Arx::new(&path)?;

        let mut s = Self {
            archive,
            path,
            roots: vec![],
            entry_list: vec![],
        };
        s.update_entries()?;
        Ok(s)
    }

    pub fn root(&self) -> Option<(jbk::EntryRange, String)> {
        self.roots.last().cloned()
    }

    pub fn enter_in(&mut self, new_root: (jbk::EntryRange, String)) -> Result<(), ArxError> {
        self.roots.push(new_root);
        self.update_entries()
    }

    pub fn jump_off(&mut self, index: usize) -> Result<(), ArxError> {
        let _ = self.roots.split_off(index);
        self.update_entries()
    }

    fn update_entries(&mut self) -> Result<(), libarx::ArxError> {
        let builder = libarx::RealBuilder::<libarx::FullBuilder>::new(&self.archive.properties);
        let read_entry = match self.root() {
            None => libarx::ReadEntry::new(&self.archive.root_index, &builder),
            Some(r) => libarx::ReadEntry::new(&r.0, &builder),
        };
        self.entry_list = read_entry.collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }
}

#[derive(Default)]
pub struct AppModel {
    archive: Option<ArxModel>,
    status_message: String,
    error_msg: ErrorMsg,
}

impl AppModel {
    fn new(path: Option<String>) -> Self {
        let mut s: Self = Default::default();
        if let Some(path) = path {
            s.load_archive(path.into());
        }
        s
    }

    fn load_archive(&mut self, path: PathBuf) {
        self.status_message = format!("Loading {}...", path.display());
        self.archive = self.error_msg.catch(ArxModel::open(path));
    }

    fn extract_all(&mut self) -> AnyResult<()> {
        if let Some(archive) = &self.archive {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                self.status_message = format!("Extracting {}...", archive.path.display());
                std::fs::create_dir_all(&folder)?;
                ExtractBuilder::new(&folder)
                    .overwrite(libarx::Overwrite::Skip)
                    .extract(&archive.archive, None)?;
                self.status_message = format!("{} extracted..", archive.path.display());
            }
        }
        Ok(())
    }

    fn has_archive(&self) -> bool {
        self.archive.is_some()
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

            ui.add_enabled_ui(self.model.has_archive(), |ui| {
                if ui.button("📦 Extract All").clicked() {
                    let result = self.model.extract_all();
                    self.model.error_msg.catch(result);
                }
            });
        });
    }

    fn breadcrumbs(&mut self, ui: &mut Ui) {
        if let Some(archive) = self.model.archive.as_mut() {
            let mut to_split = None;
            ui.horizontal(|ui| {
                if ui.button("📂").clicked() {
                    to_split = Some(0);
                }

                for (idx, dir) in archive.roots.iter().enumerate() {
                    ui.label("/");
                    if ui.button(&dir.1).clicked() {
                        to_split = Some(idx + 1);
                    }
                }
            });
            if let Some(to_split) = to_split {
                self.model.error_msg.catch(archive.jump_off(to_split));
            }
        }
    }

    fn file_list(&mut self, ui: &mut Ui) {
        if let Some(archive) = self.model.archive.as_mut() {
            let mut new_root = None;
            let mut entry_iter = archive.entry_list.iter().enumerate();
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
                        let entry = skip_iter
                            .next()
                            .expect("We should have a entry as we skip until we found our")
                            .1;
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
                                new_root = Some((*r, path.to_string()));
                            }
                        }
                    });
                });
            if let Some(new_root) = new_root {
                self.model.error_msg.catch(archive.enter_in(new_root));
            }
        }
    }

    fn status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.model.status_message);

            if let Some(archive) = self.model.archive.as_ref() {
                ui.separator();
                ui.label(format!(
                    "Archive: {}",
                    archive
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
