use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use jbk::creator::{BasicCreator, CachedContentAdder, ConcatMode, ContentAdder};

use super::{EntryStoreCreator, EntryTrait, Void};

pub struct SimpleCreator {
    cached_content_creator: CachedContentAdder<BasicCreator>,
    entry_store_creator: Box<EntryStoreCreator>,
}

impl SimpleCreator {
    pub fn new<P: AsRef<Path>>(
        outfile: P,
        concat_mode: ConcatMode,
        progress: Arc<dyn jbk::creator::Progress>,
        cache_progress: Rc<dyn jbk::creator::CacheProgress>,
        compression: jbk::creator::Compression,
    ) -> jbk::Result<Self> {
        let basic_creator = BasicCreator::new(
            outfile,
            concat_mode,
            crate::VENDOR_ID,
            compression,
            progress,
        )?;

        let entry_store_creator = Box::new(EntryStoreCreator::new());

        let cached_content_creator = CachedContentAdder::new(basic_creator, cache_progress);

        Ok(Self {
            cached_content_creator,
            entry_store_creator,
        })
    }

    pub fn finalize(self, outfile: &Path) -> Void {
        self.cached_content_creator
            .into_inner()
            .finalize(outfile, self.entry_store_creator, vec![])
    }

    pub fn adder(&mut self) -> &mut impl ContentAdder {
        &mut self.cached_content_creator
    }

    pub fn add_entry<E: EntryTrait>(&mut self, entry: &E) -> Void {
        self.entry_store_creator.add_entry(entry)
    }
}
