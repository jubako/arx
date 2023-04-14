use crate::common::{Arx, Comparator, Entry, FullBuilder, RealBuilder};
use jubako as jbk;
use jubako::reader::Range;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub fn locate<P, B>(arx: &Arx, path: P) -> jbk::Result<Entry<B::Entry>>
where
    P: AsRef<Path>,
    B: FullBuilder,
{
    let root_index = arx.root_index()?;
    let properties = arx.create_properties(&root_index)?;
    let comparator = Comparator::new(&properties);
    let builder = RealBuilder::<B>::new(&properties);
    let mut current_range: jbk::EntryRange = (&root_index).into();
    let mut components = path.as_ref().iter().peekable();
    while let Some(component) = components.next() {
        // Search for the current component.
        // All children of a parent are stored concatened.
        // So if parent_id is different than current_parent,
        // we know we are out of the directory
        let comparator = comparator.compare_with(component.as_bytes());
        let found = current_range.find(&comparator)?;
        match found {
            None => return Err("Cannot found entry".to_string().into()),
            Some(idx) => {
                let entry = current_range.get_entry(&builder, idx)?;
                if components.peek().is_none() {
                    // We have the last component
                    return Ok(entry);
                } else if let Entry::Dir(range, _) = entry {
                    current_range = range;
                } else {
                    return Err("Cannot found entry".to_string().into());
                }
            }
        }
    }
    unreachable!();
}
