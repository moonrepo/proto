#![allow(unused)]

use crate::tool_context::ToolContext;
#[cfg(feature = "otel")]
use opentelemetry::{KeyValue, global};
use std::time::{Duration, Instant};
use warpgate::{LoadedPlugin, PluginLocator};

const METER_NAME: &str = "proto";

pub struct MetricTimer {
    enabled: bool,
    start: Instant,
}

impl MetricTimer {
    pub fn start(enabled: bool) -> Self {
        Self {
            enabled,
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn record_tool_install<T, R>(
        self,
        context: &ToolContext,
        strategy: &'static str,
        cache: &'static str,
        result: Result<T, R>,
    ) -> Result<T, R> {
        #[cfg(feature = "otel")]
        if self.enabled {
            record_tool_install(context, strategy, status(&result), cache, self.elapsed());
        }

        result
    }

    pub fn record_tool_install_step<T, R>(
        self,
        context: &ToolContext,
        step: &'static str,
        result: Result<T, R>,
    ) -> Result<T, R> {
        #[cfg(feature = "otel")]
        if self.enabled {
            record_tool_install_step(context, step, status(&result), self.elapsed());
        }

        result
    }

    pub fn record_tool_uninstall<R>(
        self,
        context: &ToolContext,
        scope: &'static str,
        cache: &'static str,
        result: Result<bool, R>,
    ) -> Result<bool, R> {
        #[cfg(feature = "otel")]
        if self.enabled {
            record_tool_uninstall(
                context,
                scope,
                match &result {
                    Ok(false) => "skipped",
                    Ok(true) => "success",
                    Err(_) => "error",
                },
                cache,
                self.elapsed(),
            );
        }

        result
    }

    pub fn record_plugin_load<R>(
        self,
        context: &ToolContext,
        locator: &PluginLocator,
        result: Result<LoadedPlugin, R>,
    ) -> Result<LoadedPlugin, R> {
        #[cfg(feature = "otel")]
        if self.enabled {
            record_plugin_load(
                context,
                locator,
                status(&result),
                result
                    .as_ref()
                    .map(|loaded| cache_status(loaded.cached))
                    .unwrap_or("unknown"),
                self.elapsed(),
            );
        }

        result
    }

    pub fn record_plugin_create<T, R>(
        self,
        context: &ToolContext,
        locator: &PluginLocator,
        result: Result<T, R>,
    ) -> Result<T, R> {
        #[cfg(feature = "otel")]
        if self.enabled {
            record_plugin_create(context, locator, status(&result), self.elapsed());
        }

        result
    }
}

pub fn status<T, E>(result: &Result<T, E>) -> &'static str {
    if result.is_ok() { "success" } else { "error" }
}

pub fn cache_status(cached: bool) -> &'static str {
    if cached { "hit" } else { "miss" }
}

#[cfg(feature = "otel")]
fn locator_kind(locator: &PluginLocator) -> &'static str {
    match locator {
        PluginLocator::Data(_) => "data",
        PluginLocator::File(_) => "file",
        PluginLocator::GitHub(_) => "github",
        PluginLocator::Registry(_) => "oci",
        PluginLocator::Url(_) => "url",
    }
}

#[cfg(feature = "otel")]
fn record_counter(name: &'static str, description: &'static str, attrs: Vec<KeyValue>) {
    record_metric_probe(name);

    global::meter(METER_NAME)
        .u64_counter(name)
        .with_description(description)
        .build()
        .add(1, &attrs);
}

#[cfg(feature = "otel")]
fn record_duration(
    name: &'static str,
    description: &'static str,
    duration: Duration,
    attrs: Vec<KeyValue>,
) {
    record_metric_probe(name);

    global::meter(METER_NAME)
        .u64_histogram(name)
        .with_unit("ms")
        .with_description(description)
        .build()
        .record(duration.as_millis() as u64, &attrs);
}

#[cfg(feature = "otel")]
fn record_metric_probe(name: &'static str) {
    use std::io::Write;

    let Ok(path) = std::env::var("PROTO_TEST_OTEL_METRICS_FILE") else {
        return;
    };

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{name}");
    }
}

#[cfg(feature = "otel")]
fn record_tool_install(
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

#[cfg(feature = "otel")]
fn record_tool_install_step(
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

#[cfg(feature = "otel")]
fn record_tool_uninstall(
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

#[cfg(feature = "otel")]
fn record_plugin_load(
    context: &ToolContext,
    locator: &PluginLocator,
    status: &'static str,
    cache: &'static str,
    duration: Duration,
) {
    let attrs = vec![
        KeyValue::new("plugin", context.to_string()),
        KeyValue::new("locator", locator_kind(locator)),
        KeyValue::new("status", status),
        KeyValue::new("cache", cache),
    ];

    record_duration(
        "proto.plugin.load.duration",
        "Duration of proto plugin load",
        duration,
        attrs,
    );
}

#[cfg(feature = "otel")]
fn record_plugin_create(
    context: &ToolContext,
    locator: &PluginLocator,
    status: &'static str,
    duration: Duration,
) {
    let attrs = vec![
        KeyValue::new("plugin", context.to_string()),
        KeyValue::new("locator", locator_kind(locator)),
        KeyValue::new("status", status),
    ];

    record_duration(
        "proto.plugin.create.duration",
        "Duration of proto plugin create",
        duration,
        attrs,
    );
}
