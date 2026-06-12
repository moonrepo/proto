// raw line
// success message
// success items
// error message
// custom json
// custom jsx
// table data

use starbase_console::{Console, ConsoleStream, ConsoleStreamType, Reporter};

pub type ProtoConsole = Console<ProtoReporter>;

pub enum ReporterMode {
    Text,
    Json,
    NdJson,
}

#[derive(Debug)]
pub struct ProtoReporter {
    err: ConsoleStream,
    out: ConsoleStream,
    test_mode: bool,
}

impl ProtoReporter {
    pub fn new_testing() -> Self {
        Self {
            err: ConsoleStream::new_testing(ConsoleStreamType::Stderr),
            out: ConsoleStream::new_testing(ConsoleStreamType::Stdout),
            test_mode: true,
        }
    }
}

impl Default for ProtoReporter {
    fn default() -> Self {
        Self {
            err: ConsoleStream::empty(ConsoleStreamType::Stderr),
            out: ConsoleStream::empty(ConsoleStreamType::Stdout),
            test_mode: false,
        }
    }
}

impl Reporter for ProtoReporter {
    fn inherit_streams(&mut self, err: ConsoleStream, out: ConsoleStream) {
        if !self.test_mode {
            self.err = err;
            self.out = out;
        }
    }
}
