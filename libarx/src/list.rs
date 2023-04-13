use crate::common::*;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

enum Entry {
    File(Vec<u8>),
    Link(Vec<u8>),
    Dir(Vec<u8>, jbk::EntryRange),
}

struct EntryBuilder {
    store: Rc<jbk::reader::EntryStore>,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    path_property: jbk::reader::builder::ArrayProperty,
    first_child_property: jbk::reader::builder::IntProperty,
    nb_children_property: jbk::reader::builder::IntProperty,
}

impl EntryBuilder {
    fn new(properties: &AllProperties) -> Self {
        Self {
            store: Rc::clone(&properties.store),
            variant_id_property: properties.variant_id_property,
            path_property: properties.path_property.clone(),

            first_child_property: properties.dir_first_child_property.clone(),
            nb_children_property: properties.dir_nb_children_property.clone(),
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
            EntryType::File => Entry::File(path),
            EntryType::Link => Entry::Link(path),
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

struct Lister {
    arx: Arx,
    builder: EntryBuilder,
}

impl Lister {
    fn new(arx: Arx) -> jbk::Result<Self> {
        let properties = arx.create_properties(&arx.get_index_for_name("arx_root")?)?;
        let builder = EntryBuilder::new(&properties);
        Ok(Self { arx, builder })
    }

    fn run(&self) -> jbk::Result<()> {
        let mut current_path = LightPath::new();
        self._run(&self.arx.root_index()?, &mut current_path)
    }

    fn _run<R: Range>(&self, range: &R, current_path: &mut LightPath) -> jbk::Result<()> {
        let read_entry = ReadEntry::new(range, &self.builder);
        for entry in read_entry {
            match entry? {
                Entry::File(path) | Entry::Link(path) => {
                    current_path.println2(&path)?;
                }
                Entry::Dir(path, range) => {
                    current_path.push(OsString::from_vec(path));
                    current_path.println()?;
                    self._run(&range, current_path)?;
                    current_path.pop();
                }
            }
        }
        Ok(())
    }
}

pub fn list<P: AsRef<Path>>(infile: P) -> jbk::Result<()> {
    let arx = Arx::new(infile)?;
    let lister = Lister::new(arx)?;
    lister.run()
}
