use anyhow::Result as AnyResult;
use egui::{global_theme_preference_switch, Context, Layout, Popup, Sense, Ui};
use egui_async::{Bind, EguiAsyncPlugin};
use egui_extras::{Column, TableBuilder};
use jbk::{reader::MayMissPack, EntryRange};
use libarx::{Arx, ArxError, ArxFormatError, CommonEntry, ExtractBuilder, FileEntry, FullEntry};
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

trait Actionner {
    type Action;
    fn trigger(&mut self, action: Action);
}

impl Actionner for Option<Action> {
    type Action = Action;
    fn trigger(&mut self, action: Action) {
        *self = Some(action)
    }
}

trait Widget {
    type Action;

    fn interact(&self, ui: &mut Ui, actionner: &mut dyn Actionner<Action = Self::Action>);
}

trait Model {
    type Action;
    fn update(&mut self, action: Action);
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

impl Model for AppModel {
    type Action = Action;
    fn update(&mut self, action: Action) {
        match action {
            Action::Enter(new_root) => {
                let result = self
                    .archive
                    .as_mut()
                    .map(|a| Arc::get_mut(a).unwrap().enter_in(new_root));
                result.map(|result| self.error_msg.catch(result));
            }
            Action::Open(f) => {
                let result = self.extract_and_open(f);
                self.error_msg.catch(result);
            }
            Action::LoadArchive(path) => {
                self.load_archive(path);
            }
            Action::JumpTo(index) => {
                let result = self
                    .archive
                    .as_mut()
                    .map(|a| Arc::get_mut(a).unwrap().jump_off(index));
                result.map(|result| self.error_msg.catch(result));
            }
            Action::ExtractAll => {
                let result = self.extract_all();
                self.error_msg.catch(result);
            }
            Action::ExtractOne(entry) => {
                let result = self.extract_one(entry);
                self.error_msg.catch(result);
            }
            Action::ExtractDir((r, n)) => {
                let result = self.extract_dir(r, n);
                self.error_msg.catch(result);
            }
        }
    }
}

struct MenuBar {
    has_archive: bool,
}

impl MenuBar {
    fn new(model: &AppModel) -> Self {
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

struct StatusBar<'a> {
    status_message: &'a str,
    archive_file_name: Option<String>,
}

impl<'a> StatusBar<'a> {
    fn new(model: &'a AppModel) -> Self {
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

struct Breadcrubms<'a> {
    roots: &'a [(EntryRange, String)],
}

impl<'a> Breadcrubms<'a> {
    fn new(model: &'a ArxModel) -> Self {
        Self {
            roots: &model.roots,
        }
    }
}

impl Widget for Breadcrubms<'_> {
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

struct Spinner;

impl Widget for Spinner {
    type Action = Action;
    fn interact(&self, ui: &mut Ui, _actionner: &mut dyn Actionner<Action = Self::Action>) {
        egui::Modal::new(egui::Id::new("Spinner")).show(ui.ctx(), |ui| ui.spinner());
    }
}

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

struct FileList<'a> {
    entry_list: &'a [FullEntry],
}

impl<'a> FileList<'a> {
    fn new(model: &'a ArxModel) -> Self {
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
                Breadcrubms::new(model).interact(ui, &mut action);
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
