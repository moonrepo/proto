use crate::funcs::get_plugin_id;
use extism_pdk::{debug, error, info, trace, warn};
use std::fmt;
use tracing::{Event, Metadata, Subscriber, field::Visit, subscriber::set_global_default};
use tracing_subscriber::{Layer, Registry, layer::Context, prelude::*};

pub use tracing::level_filters::LevelFilter;

struct FieldVisitor {
    message: String,
    fields: Vec<String>,
}

impl FieldVisitor {
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

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.record_debug(field, &value)
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push(format!("{}={value:?}", field.name()));
        }
    }
}

impl fmt::Display for FieldVisitor {
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
    /// The minimum log level.
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
        let mut visitor = FieldVisitor::new(meta);

        event.record(&mut visitor);

        if *level == LevelFilter::TRACE {
            trace!("{visitor}")
        } else if *level == LevelFilter::DEBUG {
            debug!("{visitor}")
        } else if *level == LevelFilter::INFO {
            info!("{visitor}")
        } else if *level == LevelFilter::WARN {
            warn!("{visitor}")
        } else if *level == LevelFilter::ERROR {
            error!("{visitor}")
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
