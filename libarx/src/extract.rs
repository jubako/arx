use crate::common::*;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::{create_dir, create_dir_all, File};
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::rc::Rc;

enum Entry {
    File(Vec<u8>, jbk::ContentAddress),
    Link(Vec<u8>, Vec<u8>),
    Dir(Vec<u8>, jbk::EntryRange),
}

struct EntryBuilder {
    store: Rc<jbk::reader::EntryStore>,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    path_property: jbk::reader::builder::ArrayProperty,
    first_child_property: jbk::reader::builder::IntProperty,
    nb_children_property: jbk::reader::builder::IntProperty,
    content_address_property: jbk::reader::builder::ContentProperty,
    link_property: jbk::reader::builder::ArrayProperty,
}

impl EntryBuilder {
    pub fn new(properties: &AllProperties) -> Self {
        Self {
            store: Rc::clone(&properties.store),
            variant_id_property: properties.variant_id_property,
            path_property: properties.path_property.clone(),
            first_child_property: properties.dir_first_child_property.clone(),
            nb_children_property: properties.dir_nb_children_property.clone(),
            content_address_property: properties.file_content_address_property,
            link_property: properties.link_target_property.clone(),
        }
    }
}

impl jbk::reader::builder::BuilderTrait for EntryBuilder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let reader = self.store.get_entry_reader(idx);
        let path_prop = self.path_property.create(&reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        let file_type = self.variant_id_property.create(&reader)?.try_into()?;
        Ok(match file_type {
            EntryType::File => {
                let content_address = self.content_address_property.create(&reader)?;
                Entry::File(path, content_address)
            }
            EntryType::Link => {
                let target = self.link_property.create(&reader)?;
                let mut vec = vec![];
                target.resolve_to_vec(&mut vec)?;
                Entry::Link(path, vec)
            }
            EntryType::Dir => {
                let first_child: jbk::EntryIdx =
                    (self.first_child_property.create(&reader)? as u32).into();
                let nb_children: jbk::EntryCount =
                    (self.nb_children_property.create(&reader)? as u32).into();
                let range = jbk::EntryRange::new_from_size(first_child, nb_children);
                Entry::Dir(path, range)
            }
        })
    }
}

struct Extractor {
    arx: Arx,
    builder: EntryBuilder,
    current_path: RefCell<LightPath>,
}

impl Extractor {
    pub fn new(arx: Arx, extract_dir: OsString) -> jbk::Result<Self> {
        let properties = arx.create_properties(&arx.get_index_for_name("arx_root")?)?;
        let builder = EntryBuilder::new(&properties);

        Ok(Self {
            arx,
            builder,
            current_path: RefCell::new(extract_dir.into()),
        })
    }

    fn run(&mut self) -> jbk::Result<()> {
        create_dir_all(PathBuf::from(&*self.current_path.borrow()))?;
        self._run(&self.arx.root_index()?)
    }

    fn _run<R: Range>(&self, range: &R) -> jbk::Result<()> {
        let read_entry = ReadEntry::new(range, &self.builder);
        for entry in read_entry {
            match entry? {
                Entry::File(path, content_address) => {
                    self.on_file(&mut self.current_path.borrow_mut(), path, content_address)?
                }
                Entry::Link(path, target) => {
                    self.on_link(&mut self.current_path.borrow_mut(), path, target)?
                }
                Entry::Dir(path, range) => {
                    self.current_path
                        .borrow_mut()
                        .push(OsString::from_vec(path));
                    create_dir(PathBuf::from(&*self.current_path.borrow()))?;
                    self._run(&range)?;
                    self.current_path.borrow_mut().pop();
                }
            }
        }
        Ok(())
    }
    fn on_file(
        &self,
        current_path: &mut LightPath,
        entry_path: Vec<u8>,
        content_address: jbk::reader::ContentAddress,
    ) -> jbk::Result<()> {
        let reader = self.arx.container.get_reader(content_address)?;
        current_path.push(OsString::from_vec(entry_path));
        let mut file = File::create(&PathBuf::from(&*current_path))?;
        let size = reader.size().into_usize();
        let mut offset = 0;
        loop {
            let sub_size = std::cmp::min(size - offset, 4 * 1024);
            let reader = reader.into_memory_reader(offset.into(), jbk::End::new_size(sub_size))?;
            let written = file.write(reader.get_slice(jbk::Offset::zero(), jbk::End::None)?)?;
            offset += written;
            if offset == size {
                break;
            }
        }
        current_path.pop();
        Ok(())
    }

    fn on_link(
        &self,
        current_path: &mut LightPath,
        entry_path: Vec<u8>,
        target: Vec<u8>,
    ) -> jbk::Result<()> {
        current_path.push(OsString::from_vec(entry_path));
        symlink(
            PathBuf::from(OsString::from_vec(target)),
            PathBuf::from(&*current_path),
        )?;
        current_path.pop();
        Ok(())
    }
}

pub fn extract<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let mut runner = Extractor::new(arx, outdir.as_ref().into())?;
    runner.run()
}
