use starbase_utils::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
pub struct LogWriter {
    buffer: Mutex<Vec<String>>,
}

impl LogWriter {
    pub fn add_header(&self, depth: u8, title: impl AsRef<str>) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push(format!("{} {}", "#".repeat(depth as usize), title.as_ref()));
        buffer.push("".into());
    }

    pub fn add_code(&self, label: impl AsRef<str>, value: impl AsRef<str>) {
        let mut buffer = self.buffer.lock().unwrap();
        let value = value.as_ref().trim();

        buffer.push(format!("**{}**:", label.as_ref().to_uppercase()));

        if !value.is_empty() {
            buffer.push("```".into());
            buffer.push(value.into());
            buffer.push("```".into());
        }

        buffer.push("".into());
    }

    // pub fn add_line(&self) {
    //     let mut buffer = self.buffer.lock().unwrap();
    //     buffer.push("".into());
    // }

    pub fn add_value(&self, label: impl AsRef<str>, value: impl AsRef<str>) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push(format!(
            "**{}**: {}",
            label.as_ref().to_uppercase(),
            value.as_ref()
        ));
        buffer.push("".into());
    }

    pub fn add_value_opt<T: AsRef<str>>(&self, label: impl AsRef<str>, value: Option<T>) {
        if let Some(value) = value {
            self.add_value(label, value);
        }
    }

    pub fn write_to(&self, path: PathBuf) -> miette::Result<()> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push("".into());

        fs::write_file(path, buffer.join("\n"))?;

        Ok(())
    }
}
