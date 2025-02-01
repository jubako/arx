use std::io::Write;

#[derive(Clone, Debug)]
pub struct LightPath(Vec<jbk::SmallBytes>);

impl LightPath {
    pub fn new() -> Self {
        Self(Vec::with_capacity(10))
    }

    pub fn push(&mut self, component: jbk::SmallBytes) {
        self.0.push(component);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn println2(&self, component: &[u8], output: &mut impl Write) -> std::io::Result<()> {
        let mut parts = self.0.iter();
        if let Some(part) = parts.next() {
            output.write_all(part)?;
            for part in parts {
                output.write_all(b"/")?;
                output.write_all(part)?;
            }
            if !component.is_empty() {
                output.write_all(b"/")?;
            }
        }
        output.write_all(component)?;
        output.write_all(b"\n")?;
        Ok(())
    }

    pub fn println(&self, output: &mut impl Write) -> std::io::Result<()> {
        self.println2(b"", output)
    }
}

impl Default for LightPath {
    fn default() -> Self {
        Self::new()
    }
}

impl From<jbk::SmallBytes> for LightPath {
    fn from(s: jbk::SmallBytes) -> Self {
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
