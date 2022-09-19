use crate::common::{Entry, EntryKind};
use jubako as jbk;
use std::fs::{create_dir, create_dir_all, File};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub fn extract<P: AsRef<Path>>(infile: P, outdir: P) -> jbk::Result<()> {
    let container = jbk::reader::Container::new(&infile)?;
    let directory = container.get_directory_pack()?;
    let index = directory.get_index_from_name("root")?;
    let key_storage = directory.get_key_storage();
    let entry_count = index.entry_count();

    struct RangeLoop<'a> {
        f: &'a dyn Fn(&RangeLoop, u32, u32, PathBuf) -> jbk::Result<()>,
    }
    let range_loop = RangeLoop {
        f: &|range_loop, min, max, current_path| {
            for idx in min..max {
                let entry = Entry::new(index.get_entry(jbk::Idx(idx))?, &key_storage);
                let target_path = current_path.join(entry.get_path()?);
                match &entry.get_type() {
                    EntryKind::File => {
                        let content_address = entry.get_content_address();
                        let reader = container.get_reader(content_address)?;
                        let mut target_file = File::create(target_path)?;
                        std::io::copy(&mut reader.create_stream_all(), &mut target_file)?;
                    }
                    EntryKind::Directory => {
                        create_dir(&target_path)?;
                        let first_child = entry.get_first_child().0;
                        let nb_children = entry.get_nb_children().0;
                        (range_loop.f)(
                            &range_loop,
                            first_child,
                            first_child + nb_children,
                            target_path,
                        )?;
                    }
                    EntryKind::Link => {
                        let target = entry.get_target_link()?;
                        symlink(target, target_path)?;
                    }
                }
            }
            Ok(())
        },
    };
    create_dir_all(&outdir)?;
    (range_loop.f)(&range_loop, 0, entry_count.0, outdir.as_ref().to_path_buf())
}
