use flow2flow_contract::FlowValidationError;
use flow2flow_runtime::{runtime_from_steps, FlowRuntime};

pub fn weather_flow() -> Result<FlowRuntime, FlowValidationError> {
    runtime_from_steps(
        "weather",
        1,
        &[
            ("fetch", "Pull weather telemetry"),
            ("enrich", "Aggregate data into summary"),
            ("render", "Render client payload"),
        ],
    )
}

pub fn order_flow() -> Result<FlowRuntime, FlowValidationError> {
    runtime_from_steps(
        "orders",
        1,
        &[
            ("ingest", "Receive order request"),
            ("reserve", "Reserve inventory"),
            ("confirm", "Send confirmation"),
        ],
    )
}

pub fn faq_flow() -> Result<FlowRuntime, FlowValidationError> {
    runtime_from_steps(
        "faq",
        1,
        &[("lookup", "Retrieve FAQ text"), ("format", "Format for channel")],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_flow_executes() {
        let runtime = weather_flow().expect("valid weather flow");
        let outcome = runtime.execute("Lisbon");
        assert!(outcome.succeeded);
        assert_eq!(outcome.trace.len(), 3);
    }
}
