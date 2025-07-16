use extism_pdk::{debug, error, info, trace, warn};
use std::fmt;
use tracing::{
    Event, Subscriber, field::Visit, level_filters::LevelFilter, subscriber::set_global_default,
};
use tracing_subscriber::{Layer, Registry, layer::Context, prelude::*};

#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: Vec<String>,
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
            write!(f, " ({})", self.fields.join(" "))?;
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct ExtismTracingLayer {}

impl<S: Subscriber> Layer<S> for ExtismTracingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level();
        let mut visitor = FieldVisitor::default();

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

pub fn initialize_tracing() {
    set_global_default(Registry::default().with(ExtismTracingLayer::default())).unwrap()
}
