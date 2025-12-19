use crate::{LogLevel, Xlog};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Clone)]
pub struct XlogLayerConfig {
    pub enabled: bool,
    pub level: LogLevel,
    pub tag: Option<String>,
    pub include_spans: bool,
}

impl XlogLayerConfig {
    pub fn new(level: LogLevel) -> Self {
        Self {
            enabled: true,
            level,
            tag: None,
            include_spans: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn include_spans(mut self, include: bool) -> Self {
        self.include_spans = include;
        self
    }
}

#[derive(Clone)]
pub struct XlogLayerHandle {
    state: Arc<LayerState>,
}

impl XlogLayerHandle {
    pub fn set_enabled(&self, enabled: bool) {
        self.state.enabled.store(enabled, Ordering::Release);
    }

    pub fn enabled(&self) -> bool {
        self.state.enabled.load(Ordering::Acquire)
    }

    pub fn set_level(&self, level: LogLevel) {
        self.state.logger.set_level(level);
        self.state.level.store(level_to_u8(level), Ordering::Release);
    }

    pub fn level(&self) -> LogLevel {
        level_from_u8(self.state.level.load(Ordering::Acquire))
    }
}

pub struct XlogLayer {
    state: Arc<LayerState>,
    tag: Option<String>,
    include_spans: bool,
}

impl XlogLayer {
    pub fn new(logger: Xlog) -> (Self, XlogLayerHandle) {
        let config = XlogLayerConfig::new(logger.level());
        Self::with_config(logger, config)
    }

    pub fn with_config(logger: Xlog, config: XlogLayerConfig) -> (Self, XlogLayerHandle) {
        logger.set_level(config.level);
        let state = Arc::new(LayerState::new(logger, config.enabled, config.level));
        let layer = Self {
            state: Arc::clone(&state),
            tag: config.tag,
            include_spans: config.include_spans,
        };
        let handle = XlogLayerHandle { state };
        (layer, handle)
    }

    pub fn handle(&self) -> XlogLayerHandle {
        XlogLayerHandle {
            state: Arc::clone(&self.state),
        }
    }

    fn is_enabled_for(&self, level: LogLevel) -> bool {
        if !self.state.enabled.load(Ordering::Acquire) {
            return false;
        }
        let min_level = level_from_u8(self.state.level.load(Ordering::Acquire));
        level_rank(level) >= level_rank(min_level)
    }

    fn is_metadata_enabled(&self, metadata: &Metadata<'_>) -> bool {
        let level = tracing_level_to_log_level(metadata.level());
        level != LogLevel::None && self.is_enabled_for(level)
    }
}

impl<S> Layer<S> for XlogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        self.is_metadata_enabled(metadata)
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = tracing_level_to_log_level(metadata.level());
        if level == LogLevel::None {
            return;
        }
        if !self.is_enabled_for(level) {
            return;
        }
        if !self.state.logger.is_enabled(level) {
            return;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let mut message = visitor.finish();
        if self.include_spans {
            if let Some(scope) = ctx.event_scope(event) {
                let mut spans = String::new();
                for span in scope.from_root() {
                    if !spans.is_empty() {
                        spans.push_str(" > ");
                    }
                    spans.push_str(span.metadata().name());
                }
                if !spans.is_empty() {
                    if message.is_empty() {
                        message = spans;
                    } else {
                        message = format!("[{}] {}", spans, message);
                    }
                }
            }
        }
        if message.is_empty() {
            message = metadata.name().to_string();
        }

        let tag = self
            .tag
            .as_deref()
            .unwrap_or_else(|| metadata.target());
        let file = metadata.file().unwrap_or("<unknown>");
        let module = metadata.module_path().unwrap_or("<unknown>");
        let line = metadata.line().unwrap_or(0);

        self.state
            .logger
            .write_with_meta(level, Some(tag), file, module, line, &message);
    }
}

struct LayerState {
    enabled: AtomicBool,
    level: AtomicU8,
    logger: Xlog,
}

impl LayerState {
    fn new(logger: Xlog, enabled: bool, level: LogLevel) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            level: AtomicU8::new(level_to_u8(level)),
            logger,
        }
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl EventVisitor {
    fn finish(self) -> String {
        let mut output = String::new();
        if let Some(message) = self.message {
            output.push_str(&message);
        }
        if !self.fields.is_empty() {
            if !output.is_empty() {
                output.push(' ');
            }
            output.push('{');
            for (idx, (name, value)) in self.fields.iter().enumerate() {
                if idx > 0 {
                    output.push_str(", ");
                }
                output.push_str(name);
                output.push('=');
                output.push_str(value);
            }
            output.push('}');
        }
        output
    }

    fn record_field(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields.push((field.name().to_string(), value));
        }
    }
}

impl Visit for EventVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_field(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_field(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_field(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record_field(field, value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_field(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_field(field, format!("{value:?}"));
    }
}

fn tracing_level_to_log_level(level: &Level) -> LogLevel {
    match *level {
        Level::TRACE => LogLevel::Verbose,
        Level::DEBUG => LogLevel::Debug,
        Level::INFO => LogLevel::Info,
        Level::WARN => LogLevel::Warn,
        Level::ERROR => LogLevel::Error,
    }
}

fn level_rank(level: LogLevel) -> u8 {
    match level {
        LogLevel::Verbose => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
        LogLevel::Fatal => 5,
        LogLevel::None => 6,
    }
}

fn level_to_u8(level: LogLevel) -> u8 {
    level_rank(level)
}

fn level_from_u8(value: u8) -> LogLevel {
    match value {
        0 => LogLevel::Verbose,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        5 => LogLevel::Fatal,
        _ => LogLevel::None,
    }
}
