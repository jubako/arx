use std::io;
use std::io::Write;

#[derive(Clone, Debug)]
pub struct LightPath(Vec<Vec<u8>>);

impl LightPath {
    pub fn new() -> Self {
        Self(Vec::with_capacity(10))
    }

    pub fn push(&mut self, component: Vec<u8>) {
        self.0.push(component);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn println2(&self, component: &[u8]) -> std::io::Result<()> {
        let mut stdout = io::stdout().lock();
        let mut parts = self.0.iter();
        if let Some(part) = parts.next() {
            stdout.write_all(part)?;
            for part in parts {
                stdout.write_all(b"/")?;
                stdout.write_all(part)?;
            }
            if !component.is_empty() {
                stdout.write_all(b"/")?;
            }
        }
        stdout.write_all(component)?;
        stdout.write_all(b"\n")?;
        Ok(())
    }

    pub fn println(&self) -> std::io::Result<()> {
        self.println2(b"")
    }
}

impl Default for LightPath {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<u8>> for LightPath {
    fn from(s: Vec<u8>) -> Self {
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
            s.push(String::from_utf8_lossy(part).as_ref())
        }
        s
    }
}
