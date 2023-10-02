use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use super::{Adder, ConcatMode, EntryStoreCreator, EntryTrait, Void};

const VENDOR_ID: u32 = 0x41_52_58_00;

pub struct ContentAdder {
    content_pack: jbk::creator::CachedContentPackCreator,
}

impl ContentAdder {
    fn new(content_pack: jbk::creator::CachedContentPackCreator) -> Self {
        Self { content_pack }
    }

    fn into_inner(self) -> jbk::creator::CachedContentPackCreator {
        self.content_pack
    }
}

impl Adder for ContentAdder {
    fn add<R: jbk::creator::InputReader>(&mut self, reader: R) -> jbk::Result<jbk::ContentAddress> {
        let content_id = self.content_pack.add_content(reader)?;
        Ok(jbk::ContentAddress::new(1.into(), content_id))
    }
}

pub struct SimpleCreator {
    adder: ContentAdder,
    directory_pack: jbk::creator::DirectoryPackCreator,
    entry_store_creator: EntryStoreCreator,
    concat_mode: ConcatMode,
    out_dir: PathBuf,
    tmp_path_content_pack: tempfile::TempPath,
}

impl SimpleCreator {
    pub fn new<P: AsRef<Path>>(
        outfile: P,
        concat_mode: ConcatMode,
        progress: Arc<dyn jbk::creator::Progress>,
        cache_progress: Rc<dyn jbk::creator::CacheProgress>,
    ) -> jbk::Result<Self> {
        let outfile = outfile.as_ref();
        let out_dir = outfile.parent().unwrap().to_path_buf();

        let (tmp_content_pack, tmp_path_content_pack) =
            tempfile::NamedTempFile::new_in(&out_dir)?.into_parts();
        let content_pack = jbk::creator::ContentPackCreator::new_from_file_with_progress(
            tmp_content_pack,
            jbk::PackId::from(1),
            VENDOR_ID,
            Default::default(),
            jbk::CompressionType::Zstd,
            progress,
        )?;

        let directory_pack = jbk::creator::DirectoryPackCreator::new(
            jbk::PackId::from(0),
            VENDOR_ID,
            Default::default(),
        );

        let entry_store_creator = EntryStoreCreator::new();

        Ok(Self {
            adder: ContentAdder::new(jbk::creator::CachedContentPackCreator::new(
                content_pack,
                cache_progress,
            )),
            directory_pack,
            entry_store_creator,
            concat_mode,
            out_dir,
            tmp_path_content_pack,
        })
    }

    pub fn finalize(mut self, outfile: &Path) -> Void {
        self.entry_store_creator.finalize(&mut self.directory_pack);

        let mut container = match self.concat_mode {
            ConcatMode::NoConcat => None,
            _ => Some(jbk::creator::ContainerPackCreator::new(outfile)?),
        };

        let mut tmpfile = tempfile::NamedTempFile::new_in(&self.out_dir)?;
        let directory_pack_info = self.directory_pack.finalize(&mut tmpfile)?;

        let directory_locator = match self.concat_mode {
            ConcatMode::NoConcat => {
                let mut outfilename = outfile.file_name().unwrap().to_os_string();
                outfilename.push(".jbkd");
                let mut directory_pack_path = PathBuf::new();
                directory_pack_path.push(outfile);
                directory_pack_path.set_file_name(&outfilename);

                if let Err(e) = tmpfile.persist(directory_pack_path) {
                    return Err(e.error.into());
                };
                outfilename.into_vec()
            }
            _ => {
                let (file, _path) = tmpfile.into_parts();
                container
                    .as_mut()
                    .unwrap()
                    .add_pack(directory_pack_info.uuid, jbk::FileSource::new(file)?.into())?;
                vec![]
            }
        };

        let (content_pack_file, content_pack_info) =
            self.adder.into_inner().into_inner().finalize()?;
        let content_locator = match self.concat_mode {
            ConcatMode::OneFile => {
                container.as_mut().unwrap().add_pack(
                    content_pack_info.uuid,
                    jbk::FileSource::new(content_pack_file)?.into(),
                )?;
                vec![]
            }
            _ => {
                let mut outfilename = outfile.file_name().unwrap().to_os_string();
                outfilename.push(".jbkc");
                let mut content_pack_path = PathBuf::new();
                content_pack_path.push(outfile);
                content_pack_path.set_file_name(&outfilename);

                if let Err(e) = self.tmp_path_content_pack.persist(&content_pack_path) {
                    return Err(e.error.into());
                }
                outfilename.into_vec()
            }
        };

        let mut manifest_creator =
            jbk::creator::ManifestPackCreator::new(VENDOR_ID, Default::default());

        manifest_creator.add_pack(directory_pack_info, directory_locator);
        manifest_creator.add_pack(content_pack_info, content_locator);

        let mut tmpfile = tempfile::NamedTempFile::new_in(self.out_dir)?;
        let manifest_uuid = manifest_creator.finalize(&mut tmpfile)?;

        match self.concat_mode {
            ConcatMode::NoConcat => {
                if let Err(e) = tmpfile.persist(outfile) {
                    return Err(e.error.into());
                };
            }
            _ => {
                let (file, _path) = tmpfile.into_parts();
                container
                    .as_mut()
                    .unwrap()
                    .add_pack(manifest_uuid, jbk::FileSource::new(file)?.into())?;
                container.unwrap().finalize()?;
            }
        };

        Ok(())
    }

    pub fn adder(&mut self) -> &mut ContentAdder {
        &mut self.adder
    }

    pub fn add_entry<E: EntryTrait>(&mut self, entry: &E) -> Void {
        self.entry_store_creator.add_entry(entry)
    }
}
