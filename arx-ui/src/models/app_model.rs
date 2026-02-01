use super::{arx_model::ArxModel, Model};
use crate::app::Action;
use anyhow::Result as AnyResult;
use egui_async::Bind;
use libarx::{CommonEntry, ExtractBuilder, FileEntry};
use std::{path::PathBuf, sync::Arc};
use tokio::task::spawn_blocking;

#[derive(Default)]
pub struct ErrorMsg {
    error: Option<String>,
}

impl ErrorMsg {
    fn catch<T, E: ToString>(&mut self, result: Result<T, E>) -> Option<T> {
        if let Err(e) = &result {
            self.error = Some(e.to_string());
        }
        result.ok()
    }

    pub fn take(&mut self) -> Option<String> {
        self.error.take()
    }
}

#[derive(Default)]
pub struct AppModel {
    pub archive: Option<Arc<ArxModel>>,
    pub status_message: String,
    pub error_msg: ErrorMsg,
    pub background_task: Bind<bool, ()>,
}

impl AppModel {
    pub fn new(path: Option<String>) -> Self {
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
            let archive_clone = Arc::clone(archive);
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
            let archive_clone = Arc::clone(archive);
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
            let archive_clone = Arc::clone(archive);
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
            let archive_clone = Arc::clone(archive);
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

    pub fn has_archive(&self) -> bool {
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
