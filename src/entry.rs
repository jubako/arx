use jubako as jbk;
use std::fmt;

pub enum EntryKind {
    File,
    Directory,
    Link,
}

pub struct Entry<'a> {
    entry: jbk::reader::Entry<'a>,
    key_storage: &'a jbk::reader::KeyStorage<'a>,
}

impl<'a> Entry<'a> {
    pub fn new(
        entry: jbk::reader::Entry<'a>,
        key_storage: &'a jbk::reader::KeyStorage<'a>,
    ) -> Self {
        Self { entry, key_storage }
    }
    pub fn get_type(&self) -> EntryKind {
        match self.entry.get_variant_id() {
            0 => EntryKind::File,
            1 => EntryKind::Directory,
            2 => EntryKind::Link,
            _ => unreachable!(),
        }
    }

    pub fn get_path(&self) -> jbk::Result<String> {
        if let jbk::reader::Value::Array(path) = self.entry.get_value(0.into())? {
            let path = path.resolve_to_vec(self.key_storage)?;
            Ok(String::from_utf8(path)?)
        } else {
            panic!()
        }
    }

    pub fn get_content_address(&self) -> &jbk::reader::Content {
        assert!(self.entry.get_variant_id() == 0);
        let v = self.entry.get_value(1.into()).unwrap();
        if let jbk::reader::Value::Content(c) = v {
            c
        } else {
            panic!()
        }
    }

    pub fn get_target_link(&self) -> jbk::Result<String> {
        assert!(self.entry.get_variant_id() == 2);
        let v = self.entry.get_value(1.into())?;
        if let jbk::reader::Value::Array(target) = v {
            let target = target.resolve_to_vec(self.key_storage)?;
            Ok(String::from_utf8(target)?)
        } else {
            panic!()
        }
    }
}

impl<'a> fmt::Display for Entry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_path().unwrap())
        //write!(f, "{}", self.get_path().or(Err(fmt::Error))?)
    }
}
