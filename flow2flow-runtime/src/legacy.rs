use flow2flow_contract::{FlowDefinition, FlowStep, FlowValidationError};

/// Represents a runtime that can execute a validated flow definition.
#[derive(Debug, Clone)]
pub struct FlowRuntime {
    definition: FlowDefinition,
}

impl FlowRuntime {
    /// Construct a runtime from a flow definition, validating it on the way in.
    pub fn from_definition(definition: FlowDefinition) -> Result<Self, FlowValidationError> {
        definition.validate()?;
        Ok(Self { definition })
    }

    /// Execute the flow against some inbound payload.
    pub fn execute(&self, payload: &str) -> ExecutionOutcome {
        let trace = self
            .definition
            .steps
            .iter()
            .map(|step| format!("{}: handled {}", step.id, payload))
            .collect();

        ExecutionOutcome { flow_name: self.definition.name.clone(), trace, succeeded: true }
    }

    /// Access the definition used by this runtime.
    pub fn definition(&self) -> &FlowDefinition {
        &self.definition
    }
}

/// Result of executing a flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub flow_name: String,
    pub trace: Vec<String>,
    pub succeeded: bool,
}

impl ExecutionOutcome {
    pub fn summary(&self) -> String {
        format!("{}: {} steps (success={})", self.flow_name, self.trace.len(), self.succeeded)
    }
}

/// Quickly construct a runtime from simple step descriptions.
pub fn runtime_from_steps(
    name: &str,
    version: u32,
    steps: &[(&str, &str)],
) -> Result<FlowRuntime, FlowValidationError> {
    let steps =
        steps.iter().map(|(id, description)| FlowStep::new(*id, *description)).collect::<Vec<_>>();
    FlowRuntime::from_definition(FlowDefinition::new(name, version, steps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_validates_definition() {
        let definition = FlowDefinition::new(
            "broken",
            1,
            vec![FlowStep::new("step", "duplicate id"), FlowStep::new("step", "again")],
        );

        let result = FlowRuntime::from_definition(definition);
        assert!(matches!(
            result,
            Err(FlowValidationError::DuplicateStepId(id)) if id == "step"
        ));
    }

    #[test]
    fn runtime_executes_steps() {
        let runtime = runtime_from_steps(
            "weather",
            1,
            &[("fetch", "Fetch data"), ("render", "Render output")],
        )
        .expect("valid runtime");

        let outcome = runtime.execute("payload");

        assert!(outcome.succeeded);
        assert_eq!(outcome.trace.len(), 2);
        assert_eq!(outcome.flow_name, "weather");
        assert_eq!(outcome.summary(), "weather: 2 steps (success=true)");
    }
}
