//! The line `rypsok` writes for an event (RFC 009; RFC 020, TST-2): one JSON
//! object with `timestamp`, `level`, the fields of the event, and then the
//! fields of the spans the event is inside of, **at the top level**. A call
//! runs inside a span that carries `request_id` and `caller`, so every event
//! of the call has them without each macro naming them.
//!
//! The standard JSON layer of `tracing-subscriber` puts span fields under
//! `span` and `spans`; the identifier would then be at `.span.request_id`,
//! not where RFC 009 lists it. This formatter writes the same keys the
//! standard one wrote for an event (`timestamp`, `level`, the event's fields,
//! `message`) and hoists the span fields; an event's own field wins over a
//! span's of the same name.

use std::fmt;

use serde_json::Value;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::{JsonFields, Writer};
use tracing_subscriber::fmt::time::{FormatTime, UtcTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields, MakeWriter};
use tracing_subscriber::registry::LookupSpan;

/// The event formatter of [`super::layer`].
#[derive(Debug, Clone, Default)]
pub struct LineFormat;

/// The fields of an event as JSON values, in the order they were recorded.
struct Collect<'a>(&'a mut Vec<(String, Value)>);

impl Visit for Collect<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), Value::from(value)));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.push((field.name().to_owned(), Value::from(value)));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.push((field.name().to_owned(), Value::from(value)));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.push((field.name().to_owned(), Value::from(value)));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.push((field.name().to_owned(), Value::from(value)));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0
            .push((field.name().to_owned(), Value::from(format!("{value:?}"))));
    }
}

impl<S, N> FormatEvent<S, N> for LineFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields: Vec<(String, Value)> = Vec::new();
        let mut stamp = String::new();
        UtcTime::rfc_3339().format_time(&mut Writer::new(&mut stamp))?;
        fields.push(("timestamp".to_owned(), Value::from(stamp)));
        fields.push((
            "level".to_owned(),
            Value::from(event.metadata().level().as_str()),
        ));
        event.record(&mut Collect(&mut fields));
        // The spans from the outermost in: their fields come after the
        // event's, and a name the event already has is not repeated.
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                let extensions = span.extensions();
                let Some(formatted) = extensions.get::<FormattedFields<N>>() else {
                    continue;
                };
                if let Ok(Value::Object(map)) = serde_json::from_str(formatted.fields.as_str()) {
                    for (name, value) in map {
                        if !fields.iter().any(|(known, _)| *known == name) {
                            fields.push((name, value));
                        }
                    }
                }
            }
        }
        writer.write_char('{')?;
        for (index, (name, value)) in fields.iter().enumerate() {
            if index > 0 {
                writer.write_char(',')?;
            }
            write!(writer, "{}:{}", Value::from(name.as_str()), value)?;
        }
        writeln!(writer, "}}")
    }
}

/// The layer `init` installs, writing to `writer`: the formatter above and
/// JSON for the fields of spans (so that the formatter can read them back).
/// A test builds its subscriber from this too, so that it reads what the
/// process writes.
pub fn layer<S, W>(writer: W) -> tracing_subscriber::fmt::Layer<S, JsonFields, LineFormat, W>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'a> MakeWriter<'a> + 'static,
{
    tracing_subscriber::fmt::layer()
        .event_format(LineFormat)
        .fmt_fields(JsonFields::new())
        .with_writer(writer)
}
