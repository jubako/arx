use crate::light_path::LightPath;
use arx::CommonEntry;
use jbk::reader::builder::PropertyBuilderTrait;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

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
        current_path.push(OsString::from_vec(path.clone()));
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

impl arx::walk::Operator<PathBuf, arx::FullBuilder> for StableLister {
    fn on_start(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }
    fn on_stop(&self, _current_path: &mut PathBuf) -> jbk::Result<()> {
        Ok(())
    }
    fn on_directory_enter(&self, current_path: &mut PathBuf, dir: &arx::Dir) -> jbk::Result<bool> {
        current_path.push(OsString::from_vec(dir.path().clone()));
        println!("d {} {}", dir.mtime(), current_path.display());
        Ok(true)
    }
    fn on_directory_exit(&self, current_path: &mut PathBuf, _dir: &arx::Dir) -> jbk::Result<()> {
        current_path.pop();
        Ok(())
    }
    fn on_file(&self, current_path: &mut PathBuf, file: &arx::FileEntry) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(file.path().clone()));
        println!(
            "f {} {} {}",
            file.mtime(),
            file.size().into_u64(),
            current_path.display()
        );
        current_path.pop();
        Ok(())
    }
    fn on_link(&self, current_path: &mut PathBuf, link: &arx::Link) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(link.path().clone()));
        let target: PathBuf = OsString::from_vec(link.target().clone()).into();
        println!(
            "l {} {}->{}",
            link.mtime(),
            current_path.display(),
            target.display()
        );
        current_path.pop();
        Ok(())
    }
}

#[derive(clap::Args)]
pub struct Options {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(long = "stable-output", action)]
    stable_output: Option<u8>,
}

pub fn list(options: Options, verbose_level: u8) -> jbk::Result<()> {
    if verbose_level > 0 {
        println!("Listing entries in archive {:?}", options.infile);
    }
    let arx = arx::Arx::new(options.infile)?;
    if let Some(version) = options.stable_output {
        match version {
            1 => {
                let mut walker = arx::walk::Walker::new(&arx, Default::default());
                walker.run(&StableLister {})
            }
            _ => Err(format!("Stable version {version} not supported").into()),
        }
    } else {
        let mut walker = arx::walk::Walker::new(&arx, Default::default());
        walker.run(&Lister {})
    }
}
