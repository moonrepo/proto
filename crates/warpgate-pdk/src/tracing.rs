use crate::funcs::get_plugin_id;
use extism_pdk::{debug, error, info, trace, warn};
use std::fmt;
use tracing::{Event, Level, Metadata, Subscriber, field::Visit, subscriber::set_global_default};
use tracing_subscriber::{Layer, Registry, layer::Context, prelude::*};

pub use tracing::level_filters::LevelFilter;

struct EventMessage {
    message: String,
    fields: Vec<String>,
}

impl EventMessage {
    pub fn new(meta: &'static Metadata<'static>) -> Self {
        let mut visitor = Self {
            message: String::new(),
            fields: vec![format!("target={:?}", meta.target())],
        };

        if let Ok(id) = get_plugin_id() {
            visitor.fields.push(format!("id={id:?}"));
        }

        visitor
    }
}

impl Visit for EventMessage {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push(format!("{}={value:?}", field.name()));
        }
    }
}

impl fmt::Display for EventMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;

        if !self.fields.is_empty() {
            write!(f, " | {}", self.fields.join(" "))?;
        }

        Ok(())
    }
}

/// Options to customize `tracing` handling within Extism.
#[derive(Default)]
pub struct WarpgateTracingOptions {
    /// The minimum log level. Lower levels will be filtered.
    /// If not defined, logs all levels.
    pub level: Option<LevelFilter>,

    /// List of modules/prefixes to allow. Will match against the event's target.
    /// If not defined, allows all events to be sent.
    pub modules: Vec<String>,
}

struct WarpgateToExtismLayer {
    options: WarpgateTracingOptions,
}

impl<S: Subscriber> Layer<S> for WarpgateToExtismLayer {
    fn event_enabled(&self, event: &Event<'_>, _ctx: Context<'_, S>) -> bool {
        let meta = event.metadata();
        let level = meta.level();

        if !self.options.modules.is_empty()
            && !self
                .options
                .modules
                .iter()
                .any(|module| meta.target().contains(module))
        {
            return false;
        }

        if let Some(min_level) = &self.options.level {
            if level < min_level {
                return false;
            }
        }

        true
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level();

        let mut message = EventMessage::new(meta);
        event.record(&mut message);

        match *level {
            Level::ERROR => {
                error!("{message}");
            }
            Level::WARN => {
                warn!("{message}");
            }
            Level::INFO => {
                info!("{message}");
            }
            Level::DEBUG => {
                debug!("{message}");
            }
            Level::TRACE => {
                trace!("{message}");
            }
        };
    }
}

/// Initialize `tracing` events to be captured by Extism.
pub fn initialize_tracing() {
    initialize_tracing_with_options(WarpgateTracingOptions::default())
}

/// Initialize `tracing` events to be captured by Extism, with the provided options.
pub fn initialize_tracing_with_options(options: WarpgateTracingOptions) {
    set_global_default(Registry::default().with(WarpgateToExtismLayer { options })).unwrap()
}
