use proto_core::PluginLocator;
use starbase_styles::color::{self, OwoStyle};
use std::io::{BufWriter, StdoutLock, Write};

pub struct Printer<'std> {
    buffer: BufWriter<StdoutLock<'std>>,
    depth: u8,
}

unsafe impl<'std> Send for Printer<'std> {}
unsafe impl<'std> Sync for Printer<'std> {}

impl<'std> Printer<'std> {
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

    pub fn header<K: AsRef<str>, V: AsRef<str>>(&mut self, id: K, name: V) {
        self.indent();

        writeln!(
            &mut self.buffer,
            "{} {} {}",
            OwoStyle::new().bold().style(color::id(id.as_ref())),
            color::muted("-"),
            color::muted_light(name.as_ref()),
        )
        .unwrap();
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

    pub fn entry_list<K: AsRef<str>, I: IntoIterator<Item = V>, V: AsRef<str>>(
        &mut self,
        key: K,
        list: I,
        empty: Option<String>,
    ) {
        let items = list.into_iter().collect::<Vec<_>>();

        if items.is_empty() {
            if let Some(fallback) = empty {
                self.entry(key, fallback);
            }
        } else {
            self.indent();

            writeln!(&mut self.buffer, "{}:", key.as_ref()).unwrap();

            self.depth += 1;

            for item in items {
                self.indent();

                writeln!(&mut self.buffer, "{} {}", color::muted("-"), item.as_ref()).unwrap();
            }

            self.depth -= 1;
        }
    }

    pub fn entry_map<
        K: AsRef<str>,
        I: IntoIterator<Item = (V1, V2)>,
        V1: AsRef<str>,
        V2: AsRef<str>,
    >(
        &mut self,
        key: K,
        map: I,
        empty: Option<String>,
    ) {
        let items = map.into_iter().collect::<Vec<_>>();

        if items.is_empty() {
            if let Some(fallback) = empty {
                self.entry(key, fallback);
            }
        } else {
            self.indent();

            writeln!(&mut self.buffer, "{}:", key.as_ref()).unwrap();

            self.depth += 1;

            for item in items {
                self.indent();

                writeln!(
                    &mut self.buffer,
                    "{} {} {}",
                    item.0.as_ref(),
                    color::muted("-"),
                    item.1.as_ref()
                )
                .unwrap();
            }

            self.depth -= 1;
        }
    }

    pub fn locator<L: AsRef<PluginLocator>>(&mut self, locator: L) {
        match locator.as_ref() {
            PluginLocator::SourceFile { path, .. } => {
                self.entry("Source", color::path(path.canonicalize().unwrap()));
            }
            PluginLocator::SourceUrl { url } => {
                self.entry("Source", color::url(url));
            }
            PluginLocator::GitHub(github) => {
                self.entry("GitHub", color::label(&github.repo_slug));
                self.entry(
                    "Tag",
                    color::hash(github.tag.as_deref().unwrap_or("latest")),
                );
            }
            PluginLocator::Wapm(wapm) => {
                self.entry("Package", color::label(&wapm.package_name));
                self.entry(
                    "Release",
                    color::hash(wapm.version.as_deref().unwrap_or("latest")),
                );
            }
        };
    }
}
