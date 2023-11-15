use proto_core::PluginLocator;
use starbase_styles::color::{self, OwoStyle};
use std::io::{BufWriter, StdoutLock, Write};

pub struct Printer<'std> {
    buffer: BufWriter<StdoutLock<'std>>,
    indent: u8,
}

impl<'std> Printer<'std> {
    pub fn new() -> Self {
        let stdout = std::io::stdout();
        let buffer = BufWriter::new(stdout.lock());

        Printer { buffer, indent: 0 }
    }

    pub fn flush(&mut self) {
        write!(&mut self.buffer, "\n").unwrap();

        self.buffer.flush().unwrap();
    }

    // pub fn print<T: AsRef<str>>(&mut self, value: T) {
    //     self.indent();

    //     writeln!(&mut self.buffer, "{}", value.as_ref()).unwrap();
    // }

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

    pub fn start_section<T: AsRef<str>>(&mut self, header: T) {
        write!(&mut self.buffer, "\n",).unwrap();

        self.indent();

        writeln!(
            &mut self.buffer,
            "{}",
            OwoStyle::new()
                .bold()
                .style(color::muted_light(header.as_ref()))
        )
        .unwrap();

        self.indent += 1;
    }

    pub fn end_section(&mut self) {
        self.indent -= 1;
    }

    pub fn indent(&mut self) {
        if self.indent > 0 {
            write!(&mut self.buffer, "{}", "  ".repeat(self.indent as usize)).unwrap();
        }
    }

    pub fn entry<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) {
        self.indent();

        writeln!(&mut self.buffer, "{}: {}", key.as_ref(), value.as_ref()).unwrap();
    }

    pub fn entry_list<K: AsRef<str>, I: IntoIterator<Item = V>, V: AsRef<str>, F: AsRef<str>>(
        &mut self,
        key: K,
        list: I,
        empty: F,
    ) {
        let items = list.into_iter().collect::<Vec<_>>();

        if items.is_empty() {
            self.entry(key, empty);
        } else {
            self.indent();

            writeln!(&mut self.buffer, "{}:", key.as_ref()).unwrap();

            self.indent += 1;

            for item in items {
                self.indent();

                writeln!(&mut self.buffer, "{} {}", color::muted("-"), item.as_ref()).unwrap();
            }

            self.indent -= 1;
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
