use crate::tool_context::ToolContext;
use opentelemetry::{KeyValue, global};
use std::time::{Duration, Instant};
use warpgate::PluginLocator;

const METER_NAME: &str = "proto";

pub struct MetricTimer {
    start: Instant,
}

impl MetricTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

fn record_counter(name: &'static str, description: &'static str, attrs: Vec<KeyValue>) {
    global::meter(METER_NAME)
        .u64_counter(name)
        .with_description(description)
        .build()
        .add(1, &attrs);
}

fn record_duration(
    name: &'static str,
    description: &'static str,
    duration: Duration,
    attrs: Vec<KeyValue>,
) {
    global::meter(METER_NAME)
        .u64_histogram(name)
        .with_unit("ms")
        .with_description(description)
        .build()
        .record(duration.as_millis() as u64, &attrs);
}

pub fn status<T, E>(result: &Result<T, E>) -> &'static str {
    if result.is_ok() { "success" } else { "error" }
}

pub fn cache_status(cached: bool) -> &'static str {
    if cached { "hit" } else { "miss" }
}

fn locator_kind(locator: &PluginLocator) -> &'static str {
    match locator {
        PluginLocator::Data(_) => "data",
        PluginLocator::File(_) => "file",
        PluginLocator::GitHub(_) => "github",
        PluginLocator::Registry(_) => "oci",
        PluginLocator::Url(_) => "url",
    }
}

pub fn record_tool_install(
    context: &ToolContext,
    strategy: &'static str,
    status: &'static str,
    cache: &'static str,
    duration: Duration,
) {
    let attrs = vec![
        KeyValue::new("tool", context.to_string()),
        KeyValue::new("strategy", strategy),
        KeyValue::new("status", status),
        KeyValue::new("cache", cache),
    ];

    record_counter(
        "proto.tool.install.attempts",
        "Number of proto tool install attempts",
        attrs.clone(),
    );
    record_duration(
        "proto.tool.install.duration",
        "Duration of proto tool install",
        duration,
        attrs,
    );
}

pub fn record_tool_install_step(
    context: &ToolContext,
    step: &'static str,
    status: &'static str,
    duration: Duration,
) {
    let attrs = vec![
        KeyValue::new("tool", context.to_string()),
        KeyValue::new("step", step),
        KeyValue::new("status", status),
    ];

    record_counter(
        "proto.tool.install.step.attempts",
        "Number of proto tool install step attempts",
        attrs.clone(),
    );
    record_duration(
        "proto.tool.install.step.duration",
        "Duration of proto tool install step",
        duration,
        attrs,
    );
}

pub fn record_tool_uninstall(
    context: &ToolContext,
    scope: &'static str,
    status: &'static str,
    cache: &'static str,
    duration: Duration,
) {
    let attrs = vec![
        KeyValue::new("tool", context.to_string()),
        KeyValue::new("scope", scope),
        KeyValue::new("status", status),
        KeyValue::new("cache", cache),
    ];

    record_counter(
        "proto.tool.uninstall.attempts",
        "Number of proto tool uninstall attempts",
        attrs.clone(),
    );
    record_duration(
        "proto.tool.uninstall.duration",
        "Duration of proto tool uninstall",
        duration,
        attrs,
    );
}

pub fn record_plugin_load(
    id: impl ToString,
    locator: &PluginLocator,
    status: &'static str,
    cache: &'static str,
    duration: Duration,
) {
    let attrs = vec![
        KeyValue::new("plugin", id.to_string()),
        KeyValue::new("locator", locator_kind(locator)),
        KeyValue::new("status", status),
        KeyValue::new("cache", cache),
    ];

    record_counter(
        "proto.plugin.load.attempts",
        "Number of proto plugin load attempts",
        attrs.clone(),
    );
    record_duration(
        "proto.plugin.load.duration",
        "Duration of proto plugin load",
        duration,
        attrs,
    );
}
