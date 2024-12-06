use starbase_styles::color::{self, OwoStyle};
use std::io::{BufWriter, StdoutLock, Write};

pub struct Printer<'std> {
    buffer: BufWriter<StdoutLock<'std>>,
    depth: u8,
}

unsafe impl Send for Printer<'_> {}
unsafe impl Sync for Printer<'_> {}

impl Printer<'_> {
    pub fn new() -> Self {
        let stdout = std::io::stdout();
        let buffer = BufWriter::new(stdout.lock());

        Printer { buffer, depth: 0 }
    }

    pub fn flush(&mut self) {
        self.line();
        self.buffer.flush().unwrap();
    }

    pub fn line(&mut self) {
        writeln!(&mut self.buffer).unwrap();
    }

    pub fn section(
        &mut self,
        func: impl FnOnce(&mut Printer) -> miette::Result<()>,
    ) -> miette::Result<()> {
        self.depth += 1;
        func(self)?;
        self.depth -= 1;

        Ok(())
    }

    pub fn named_section<T: AsRef<str>>(
        &mut self,
        name: T,
        func: impl FnOnce(&mut Printer) -> miette::Result<()>,
    ) -> miette::Result<()> {
        writeln!(&mut self.buffer).unwrap();

        self.indent();

        writeln!(
            &mut self.buffer,
            "{}",
            OwoStyle::new()
                .bold()
                .style(color::muted_light(name.as_ref()))
        )
        .unwrap();

        self.section(func)?;

        Ok(())
    }

    pub fn indent(&mut self) {
        if self.depth > 0 {
            write!(&mut self.buffer, "{}", "  ".repeat(self.depth as usize)).unwrap();
        }
    }

    pub fn entry<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) {
        self.indent();

        writeln!(&mut self.buffer, "{}: {}", key.as_ref(), value.as_ref()).unwrap();
    }

    pub fn list<I: IntoIterator<Item = V>, V: AsRef<str>>(&mut self, list: I) {
        let items = list.into_iter().collect::<Vec<_>>();

        for item in items {
            self.indent();

            writeln!(&mut self.buffer, "{} {}", color::muted("-"), item.as_ref()).unwrap();
        }
    }
}
