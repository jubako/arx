use crate::light_path::LightPath;
use arx::CommonEntry;
use jbk::reader::builder::PropertyBuilderTrait;
use log::info;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

type Path = Vec<u8>;

struct PathBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl arx::Builder for PathBuilder {
    type Entry = Path;

    fn new(properties: &arx::AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &arx::Reader) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

type LightBuilder = (PathBuilder, PathBuilder, PathBuilder);

struct Lister {}

impl arx::walk::Operator<LightPath, LightBuilder> for Lister {
    fn on_start(&self, _current_path: &mut LightPath) -> jbk::Result<()> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut LightPath) -> jbk::Result<()> {
        Ok(())
    }
    fn on_directory_enter(&self, current_path: &mut LightPath, path: &Path) -> jbk::Result<bool> {
        current_path.push(path.clone());
        current_path.println()?;
        Ok(true)
    }
    fn on_directory_exit(&self, current_path: &mut LightPath, _path: &Path) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut LightPath, path: &Path) -> jbk::Result<()> {
        Ok(current_path.println2(path)?)
    }
    fn on_link(&self, current_path: &mut LightPath, path: &Path) -> jbk::Result<()> {
        Ok(current_path.println2(path)?)
    }
}

struct StableLister {}

impl arx::walk::Operator<arx::PathBuf, arx::FullBuilder> for StableLister {
    fn on_start(&self, _current_path: &mut arx::PathBuf) -> jbk::Result<()> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut arx::PathBuf) -> jbk::Result<()> {
        Ok(())
    }
    fn on_directory_enter(
        &self,
        current_path: &mut arx::PathBuf,
        dir: &arx::Dir,
    ) -> jbk::Result<bool> {
        current_path.push(String::from_utf8_lossy(dir.path()).as_ref());
        println!("d {} {}", dir.mtime(), current_path);
        Ok(true)
    }
    fn on_directory_exit(
        &self,
        current_path: &mut arx::PathBuf,
        _dir: &arx::Dir,
    ) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut arx::PathBuf, file: &arx::FileEntry) -> jbk::Result<()> {
        current_path.push(String::from_utf8_lossy(file.path()).as_ref());
        println!(
            "f {} {} {}",
            file.mtime(),
            file.size().into_u64(),
            current_path
        );
        current_path.pop();
        Ok(())
    }
    fn on_link(&self, current_path: &mut arx::PathBuf, link: &arx::Link) -> jbk::Result<()> {
        current_path.push(String::from_utf8_lossy(link.path()).as_ref());
        let target: PathBuf = String::from_utf8_lossy(link.target()).as_ref().into();
        println!("l {} {}->{}", link.mtime(), current_path, target.display());
        current_path.pop();
        Ok(())
    }
}

/// List the content in an archive.
#[derive(clap::Args, Debug)]
pub struct Options {
    /// Archive to read
    #[arg(value_parser)]
    infile: PathBuf,

    /// Use stable output (for scripting)
    #[arg(long = "stable-output", action)]
    stable_output: Option<u8>,

    #[arg(from_global)]
    verbose: u8,
}

pub fn list(options: Options) -> Result<()> {
    info!("Listing entries in archive {:?}", options.infile);
    let arx =
        arx::Arx::new(&options.infile).with_context(|| format!("Opening {:?}", options.infile))?;
    if let Some(version) = options.stable_output {
        match version {
            1 => {
                let mut walker = arx::walk::Walker::new(&arx, Default::default());
                Ok(walker.run(&StableLister {})?)
            }
            _ => Err(anyhow!("Stable version {version} not supported")),
        }
    } else {
        let mut walker = arx::walk::Walker::new(&arx, Default::default());
        Ok(walker.run(&Lister {})?)
    }
}
