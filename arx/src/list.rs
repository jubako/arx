use crate::light_path::LightPath;
use arx::{ArxError, CommonEntry};
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::ByteSlice;
use log::info;
use std::cell::RefCell;
use std::io::Write;
use std::ops::DerefMut;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, ValueHint};

type Path = jbk::SmallBytes;

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

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = jbk::SmallBytes::new();
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

type LightBuilder = (PathBuilder, PathBuilder, PathBuilder);

struct Lister<W>
where
    W: std::io::Write,
{
    output: RefCell<std::io::BufWriter<W>>,
}

impl<W> arx::walk::Operator<LightPath, LightBuilder> for Lister<W>
where
    W: std::io::Write,
{
    type Error = ArxError;
    fn on_start(&self, _current_path: &mut LightPath) -> Result<(), ArxError> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut LightPath) -> Result<(), ArxError> {
        Ok(())
    }
    fn on_directory_enter(
        &self,
        current_path: &mut LightPath,
        path: &Path,
    ) -> Result<bool, ArxError> {
        current_path.push(path.clone());
        current_path.println(self.output.borrow_mut().deref_mut())?;
        Ok(true)
    }
    fn on_directory_exit(
        &self,
        current_path: &mut LightPath,
        _path: &Path,
    ) -> Result<(), ArxError> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut LightPath, path: &Path) -> Result<(), ArxError> {
        Ok(current_path.println2(path, self.output.borrow_mut().deref_mut())?)
    }
    fn on_link(&self, current_path: &mut LightPath, path: &Path) -> Result<(), ArxError> {
        Ok(current_path.println2(path, self.output.borrow_mut().deref_mut())?)
    }
}

struct StableLister<W>
where
    W: std::io::Write,
{
    output: RefCell<std::io::BufWriter<W>>,
}

impl<W> arx::walk::Operator<arx::PathBuf, arx::FullBuilder> for StableLister<W>
where
    W: std::io::Write,
{
    type Error = ArxError;
    fn on_start(&self, _current_path: &mut arx::PathBuf) -> Result<(), ArxError> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut arx::PathBuf) -> Result<(), ArxError> {
        Ok(())
    }
    fn on_directory_enter(
        &self,
        current_path: &mut arx::PathBuf,
        dir: &arx::Dir,
    ) -> Result<bool, ArxError> {
        current_path.push(String::from_utf8_lossy(dir.path()).as_ref());
        writeln!(
            self.output.borrow_mut(),
            "d {} {}",
            dir.mtime(),
            current_path
        )?;
        Ok(true)
    }
    fn on_directory_exit(
        &self,
        current_path: &mut arx::PathBuf,
        _dir: &arx::Dir,
    ) -> Result<(), ArxError> {
        current_path.pop();
        Ok(())
    }
    fn on_file(
        &self,
        current_path: &mut arx::PathBuf,
        file: &arx::FileEntry,
    ) -> Result<(), ArxError> {
        current_path.push(String::from_utf8_lossy(file.path()).as_ref());
        writeln!(
            self.output.borrow_mut(),
            "f {} {} {}",
            file.mtime(),
            file.size().into_u64(),
            current_path
        )?;
        current_path.pop();
        Ok(())
    }
    fn on_link(&self, current_path: &mut arx::PathBuf, link: &arx::Link) -> Result<(), ArxError> {
        current_path.push(String::from_utf8_lossy(link.path()).as_ref());
        let target: PathBuf = String::from_utf8_lossy(link.target()).as_ref().into();
        writeln!(
            self.output.borrow_mut(),
            "l {} {}->{}",
            link.mtime(),
            current_path,
            target.display()
        )?;
        current_path.pop();
        Ok(())
    }
}

/// List the content in an archive.
#[derive(Parser, Debug)]
pub struct Options {
    /// Archive to read
    #[arg(value_parser, value_hint= ValueHint::FilePath)]
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
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    let handle = std::io::BufWriter::new(handle);
    if let Some(version) = options.stable_output {
        match version {
            1 => {
                let mut walker = arx::walk::Walker::new(&arx, Default::default());
                Ok(walker.run(&StableLister {
                    output: RefCell::new(handle),
                })?)
            }
            _ => Err(anyhow!("Stable version {version} not supported")),
        }
    } else {
        let mut walker = arx::walk::Walker::new(&arx, Default::default());
        Ok(walker.run(&Lister {
            output: RefCell::new(handle),
        })?)
    }
}
