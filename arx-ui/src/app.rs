use anyhow::Result as AnyResult;
use egui::{global_theme_preference_switch, Context, Layout, Popup, Sense, TextBuffer, Ui};
use egui_async::{Bind, EguiAsyncPlugin};
use egui_extras::{Column, TableBuilder};
use jbk::{reader::MayMissPack, EntryRange};
use libarx::{Arx, ArxError, ArxFormatError, CommonEntry, ExtractBuilder, FileEntry};
use std::{path::PathBuf, sync::Arc};
use tokio::task::spawn_blocking;

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
            archive: archive,
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

    fn extract(&self, f: FileEntry, outfile: &std::path::Path) -> Result<bool, libarx::ArxError> {
        let bytes = self
            .archive
            .get_bytes(f.content())?
            .and_then(|m| m.transpose())
            .ok_or(ArxFormatError(
                "Entry Content should point to valid content",
            ))?;
        match bytes {
            MayMissPack::FOUND(bytes) => {
                use std::io::Write;

                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(&outfile)?;
                let size = bytes.size().into_u64();
                let mut offset = 0;
                loop {
                    let sub_size = std::cmp::min(size - offset, 4 * 1024) as usize;
                    let written = file.write(&bytes.get_slice(offset.into(), sub_size)?)?;
                    offset += written as u64;
                    if offset == size {
                        break;
                    }
                }
                Ok(true)
            }
            MayMissPack::MISSING(_) => Ok(false),
        }
    }

    fn extract_and_open(&self, f: FileEntry) -> Result<(), libarx::ArxError> {
        let tmpdir = tempfile::tempdir()?;
        let tmp_file = tmpdir
            .keep()
            .join(String::from_utf8_lossy(f.path()).as_ref());

        if self.extract(f, &tmp_file)? {
            open::that_detached(tmp_file)?;
        }
        Ok(())
    }
}

enum Action {
    Enter((EntryRange, String)),
    Open(FileEntry),
    LoadArchive(PathBuf),
    JumpTo(usize),
    ExtractAll,
    ExtractOne(FileEntry),
    ExtractDir((EntryRange, String)),
}

#[derive(Default)]
pub struct AppModel {
    archive: Option<Arc<ArxModel>>,
    status_message: String,
    error_msg: ErrorMsg,
    background_task: Bind<bool, ()>,
}

impl AppModel {
    fn new(path: Option<String>) -> Self {
        let mut s: Self = Self {
            archive: None,
            status_message: String::new(),
            error_msg: ErrorMsg::default(),
            background_task: Bind::new(false),
        };
        if let Some(path) = path {
            s.load_archive(path.into());
        }
        s
    }

    fn load_archive(&mut self, path: PathBuf) {
        self.status_message = format!("Loading {}...", path.display());
        self.archive = self.error_msg.catch(ArxModel::open(path)).map(Arc::new);
    }

    fn extract_all(&mut self) -> AnyResult<()> {
        if let Some(archive) = &self.archive {
            let archive_clone = Arc::clone(&archive);
            self.status_message = format!("Extracting {}...", archive.path.display());
            self.background_task.request(async move {
                if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                    spawn_blocking(move || -> Result<bool, ()> {
                        std::fs::create_dir_all(folder.path()).map_err(|_| ())?;

                        ExtractBuilder::new(folder.path())
                            .overwrite(libarx::Overwrite::Skip)
                            .extract(&archive_clone.archive, None)
                            .map_err(|_| ())?;
                        Ok(true)
                    })
                    .await
                    .unwrap()
                } else {
                    Ok(true)
                }
            });
        }
        Ok(())
    }

    fn extract_dir(&mut self, range: jbk::EntryRange, dir_name: String) -> AnyResult<()> {
        if let Some(archive) = &self.archive {
            let archive_clone = Arc::clone(&archive);
            self.status_message = format!("Extracting {}...", archive.path.display());
            self.background_task.request(async move {
                if let Some(parent_folder) = rfd::AsyncFileDialog::new()
                    .set_can_create_directories(true)
                    .pick_folder()
                    .await
                {
                    let folder = parent_folder.path().join(dir_name);
                    spawn_blocking(move || {
                        std::fs::create_dir_all(&folder).map_err(|_| ())?;
                        ExtractBuilder::new(&folder)
                            .overwrite(libarx::Overwrite::Skip)
                            .extract_root(&archive_clone.archive, range)
                            .map_err(|_| ())?;
                        Ok(true)
                    })
                    .await
                    .unwrap()
                } else {
                    Ok(true)
                }
            });
        }
        Ok(())
    }

    fn extract_one(&mut self, entry: FileEntry) -> AnyResult<()> {
        if let Some(archive) = &self.archive {
            let archive_clone = Arc::clone(&archive);
            let entry_path = String::from_utf8_lossy(entry.path()).to_string();
            let entry = entry.clone();
            self.status_message = format!("Extracting {}...", entry_path);
            self.background_task.request(async move {
                if let Some(outfile) = rfd::AsyncFileDialog::new()
                    .set_file_name(entry_path)
                    .save_file()
                    .await
                {
                    spawn_blocking(move || {
                        archive_clone.extract(entry, outfile.path()).map_err(|_| ())
                    })
                    .await
                    .unwrap()
                } else {
                    Ok(false)
                }
            });
        }
        Ok(())
    }

    fn extract_and_open(&mut self, entry: FileEntry) -> AnyResult<()> {
        if let Some(archive) = &self.archive {
            let archive_clone = Arc::clone(&archive);
            self.background_task.request(async move {
                spawn_blocking(move || {
                    archive_clone.extract_and_open(entry).map_err(|_| ())?;
                    Ok(true)
                })
                .await
                .unwrap()
            });
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

    fn menubar(&self, ui: &mut Ui) -> Option<Action> {
        let mut action = None;
        egui::MenuBar::new().ui(ui, |ui| {
            if ui.button("📂 Open Archive").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Arx Archive", &["arx"])
                    .pick_file()
                {
                    action = Some(Action::LoadArchive(file));
                }
            }

            ui.add_enabled_ui(self.model.has_archive(), |ui| {
                if ui.button("📦 Extract All").clicked() {
                    action = Some(Action::ExtractAll);
                }
            });
        });
        action
    }

    fn breadcrumbs(&self, ui: &mut Ui) -> Option<Action> {
        let mut action = None;
        if let Some(archive) = &self.model.archive {
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
                action = Some(Action::JumpTo(to_split));
            }
        }
        action
    }

    fn file_list(&self, ui: &mut Ui) -> Option<Action> {
        let mut action = None;
        if let Some(archive) = &self.model.archive {
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

                        Popup::context_menu(&response).show(|ui| {
                            if ui.button("Extract").clicked() {
                                match entry {
                                    libarx::Entry::Dir(r, d) => {
                                        action = Some(Action::ExtractDir((
                                            *r,
                                            String::from_utf8_lossy(d.path()).to_string(),
                                        )))
                                    }
                                    libarx::Entry::File(f) => {
                                        action = Some(Action::ExtractOne(f.clone()));
                                    }
                                    libarx::Entry::Link(_) => {}
                                }
                            }
                        });

                        if response.double_clicked() {
                            match entry {
                                libarx::Entry::Dir(r, _) => {
                                    action = Some(Action::Enter((*r, path.to_string())));
                                }
                                libarx::Entry::File(f) => {
                                    action = Some(Action::Open(f.clone()));
                                }
                                libarx::Entry::Link(_) => {}
                            }
                        }
                    });
                });
        }
        action
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
        ctx.plugin_or_default::<EguiAsyncPlugin>();
        let mut action = None;

        action = action.or(egui::TopBottomPanel::top("menubar")
            .show(ctx, |ui| self.menubar(ui))
            .inner);

        action = action.or(egui::TopBottomPanel::bottom("status")
            .show(ctx, |ui| {
                self.status_bar(ui);
                None
            })
            .inner);

        action = action.or(egui::TopBottomPanel::top("breadcrumbs")
            .show(ctx, |ui| self.breadcrumbs(ui))
            .inner);

        action = action.or(egui::CentralPanel::default()
            .show(ctx, |ui| {
                if self.model.background_task.is_pending() {
                    egui::Modal::new(egui::Id::new("Spinner")).show(ctx, |ui| ui.spinner());
                }
                self.file_list(ui)
            })
            .inner);

        if let Some(action) = action {
            match action {
                Action::Enter(new_root) => {
                    let result = self
                        .model
                        .archive
                        .as_mut()
                        .map(|a| Arc::get_mut(a).unwrap().enter_in(new_root));
                    result.map(|result| self.model.error_msg.catch(result));
                }
                Action::Open(f) => {
                    let result = self.model.extract_and_open(f);
                    self.model.error_msg.catch(result);
                }
                Action::LoadArchive(path) => {
                    self.model.load_archive(path);
                }
                Action::JumpTo(index) => {
                    let result = self
                        .model
                        .archive
                        .as_mut()
                        .map(|a| Arc::get_mut(a).unwrap().jump_off(index));
                    result.map(|result| self.model.error_msg.catch(result));
                }
                Action::ExtractAll => {
                    let result = self.model.extract_all();
                    self.model.error_msg.catch(result);
                }
                Action::ExtractOne(entry) => {
                    let result = self.model.extract_one(entry);
                    self.model.error_msg.catch(result);
                }
                Action::ExtractDir((r, n)) => {
                    let result = self.model.extract_dir(r, n);
                    self.model.error_msg.catch(result);
                }
            }
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
