use std::fmt;
use std::sync::OnceLock;

use opentelemetry::global;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use opentelemetry_stdout::SpanExporter;
use time::macros::format_description;
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format, time::FormatTime, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::EnvFilter;

struct BracketedTime;

impl FormatTime for BracketedTime {
    fn format_time(&self, w: &mut format::Writer<'_>) -> fmt::Result {
        let now = time::OffsetDateTime::now_utc();
        write!(
            w,
            "[{}]",
            now.format(&format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
            ))
            .unwrap()
        )
    }
}

struct BracketedFormatter;

impl<S, N> FormatEvent<S, N> for BracketedFormatter
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let timer = BracketedTime;
        timer.format_time(&mut writer)?;

        write!(
            writer,
            "[{}] ",
            event.metadata().level().to_string().to_lowercase()
        )?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

static INIT_LOGGER: OnceLock<SdkTracerProvider> = OnceLock::new();

pub fn init_tracer() -> &'static SdkTracerProvider {
    INIT_LOGGER.get_or_init(|| {
        global::set_text_map_propagator(TraceContextPropagator::new());
        // Install stdout exporter pipeline to be able to retrieve the collected spans.
        // For the demonstration, use `Sampler::AlwaysOn` sampler to sample all traces.
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(SpanExporter::default())
            .build();

        global::set_tracer_provider(provider.clone());

        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .event_format(BracketedFormatter)
            .init();

        provider
    })
}
