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
        if let jbk::reader::Value::Array(path) = self.entry.get_value(0.into()).unwrap() {
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
        let v = self.entry.get_value(1.into()).unwrap();
        if let jbk::reader::Value::Array(target) = v {
            let target = target.resolve_to_vec(self.key_storage)?;
            Ok(String::from_utf8(target)?)
        } else {
            panic!()
        }
    }

    pub fn get_first_child(&self) -> jbk::Idx<u32> {
        assert!(self.entry.get_variant_id() == 1);
        let v = self.entry.get_value(1.into()).unwrap();
        jbk::Idx(match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!()
        })
    }

    pub fn get_nb_children(&self) -> jbk::Count<u32> {
        assert!(self.entry.get_variant_id() == 1);
        let v = self.entry.get_value(2.into()).unwrap();
        jbk::Count(match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!()
        })
    }
}

impl<'a> fmt::Display for Entry<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_type() {
            EntryKind::Directory => write!(f, "{}/", self.get_path().unwrap()),
            _ => write!(f, "{}", self.get_path().unwrap())
        }
        //write!(f, "{}", self.get_path().or(Err(fmt::Error))?)
    }
}
