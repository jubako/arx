use jubako as jbk;
use std::fmt;
use std::rc::Rc;

#[derive(PartialEq)]
pub enum EntryKind {
    File,
    Directory,
    Link,
}

pub struct Entry {
    entry: jbk::reader::Entry,
    key_storage: Rc<jbk::reader::KeyStorage>,
}

impl Entry {
    pub fn new(entry: jbk::reader::Entry, key_storage: Rc<jbk::reader::KeyStorage>) -> Self {
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

    pub fn is_file(&self) -> bool {
        self.entry.get_variant_id() == 0
    }

    pub fn is_dir(&self) -> bool {
        self.entry.get_variant_id() == 1
    }

    pub fn is_link(&self) -> bool {
        self.entry.get_variant_id() == 2
    }

    pub fn get_path(&self) -> jbk::Result<String> {
        if let jbk::reader::Value::Array(path) = self.entry.get_value(0.into()).unwrap() {
            let path = path.resolve_to_vec(&self.key_storage)?;
            Ok(String::from_utf8(path)?)
        } else {
            panic!()
        }
    }

    pub fn get_parent(&self) -> Option<jbk::Idx<u32>> {
        let v = self.entry.get_value(1.into()).unwrap();
        let idx = match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!(),
        };
        if idx == 0 {
            None
        } else {
            Some(jbk::Idx(idx - 1))
        }
    }

    pub fn get_content_address(&self) -> &jbk::reader::Content {
        assert!(self.is_file());
        let v = self.entry.get_value(2.into()).unwrap();
        if let jbk::reader::Value::Content(c) = v {
            c
        } else {
            panic!()
        }
    }

    pub fn get_target_link(&self) -> jbk::Result<String> {
        assert!(self.is_link());
        let v = self.entry.get_value(2.into()).unwrap();
        if let jbk::reader::Value::Array(target) = v {
            let target = target.resolve_to_vec(&self.key_storage)?;
            Ok(String::from_utf8(target)?)
        } else {
            panic!()
        }
    }

    pub fn get_first_child(&self) -> jbk::Idx<u32> {
        assert!(self.is_dir());
        let v = self.entry.get_value(2.into()).unwrap();
        jbk::Idx(match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!(),
        })
    }

    pub fn get_nb_children(&self) -> jbk::Count<u32> {
        assert!(self.is_dir());
        let v = self.entry.get_value(3.into()).unwrap();
        jbk::Count(match v {
            jbk::reader::Value::U8(v) => *v as u32,
            jbk::reader::Value::U16(v) => *v as u32,
            jbk::reader::Value::U32(v) => *v,
            _ => panic!(),
        })
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_type() {
            EntryKind::Directory => write!(f, "{}/", self.get_path().unwrap()),
            _ => write!(f, "{}", self.get_path().unwrap()),
        }
        //write!(f, "{}", self.get_path().or(Err(fmt::Error))?)
    }
}
