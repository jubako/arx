use std::ffi::OsString;
use std::io;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

#[derive(Clone, Debug)]
pub struct LightPath(Vec<OsString>);

impl LightPath {
    pub fn new() -> Self {
        Self(Vec::with_capacity(10))
    }

    pub fn push(&mut self, component: OsString) {
        self.0.push(component);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn println(&self) -> std::io::Result<()> {
        let mut stdout = io::stdout().lock();
        let mut parts = self.0.iter();
        if let Some(part) = parts.next() {
            stdout.write_all(part.as_bytes())?;
            for part in parts {
                stdout.write_all(b"/")?;
                stdout.write_all(part.as_bytes())?;
            }
        }
        stdout.write_all(b"\n")?;
        Ok(())
    }
}

impl Default for LightPath {
    fn default() -> Self {
        Self::new()
    }
}

impl From<OsString> for LightPath {
    fn from(s: OsString) -> Self {
        let mut p = Self::new();
        p.push(s);
        p
    }
}

impl From<&LightPath> for std::path::PathBuf {
    fn from(p: &LightPath) -> Self {
        let size = p.0.iter().map(|v| v.len()).sum();
        let mut s = Self::with_capacity(size);
        for part in &p.0 {
            s.push(part)
        }
        s
    }
}
