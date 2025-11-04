use std::collections::HashSet;

use crate::FlowValidationError;

/// High level description of a flow that can be executed by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowDefinition {
    pub name: String,
    pub version: u32,
    pub steps: Vec<FlowStep>,
}

impl FlowDefinition {
    /// Construct a new flow definition.
    pub fn new<N: Into<String>>(name: N, version: u32, steps: Vec<FlowStep>) -> Self {
        Self { name: name.into(), version, steps }
    }

    /// Perform basic validation on the flow.
    pub fn validate(&self) -> Result<(), FlowValidationError> {
        if self.name.trim().is_empty() {
            return Err(FlowValidationError::EmptyName);
        }

        let mut seen = HashSet::new();
        for step in &self.steps {
            if !seen.insert(step.id.clone()) {
                return Err(FlowValidationError::DuplicateStepId(step.id.clone()));
            }
        }

        Ok(())
    }
}

/// Individual step within a flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowStep {
    pub id: String,
    pub description: String,
}

impl FlowStep {
    pub fn new<I: Into<String>, D: Into<String>>(id: I, description: D) -> Self {
        Self { id: id.into(), description: description.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_succeeds_for_simple_flow() {
        let flow = FlowDefinition::new(
            "weather",
            1,
            vec![
                FlowStep::new("start", "Fetch latest weather data"),
                FlowStep::new("render", "Render summary for the client"),
            ],
        );

        assert!(flow.validate().is_ok());
    }

    #[test]
    fn validation_fails_for_duplicate_steps() {
        let flow = FlowDefinition::new(
            "dup-flow",
            1,
            vec![FlowStep::new("step", "First"), FlowStep::new("step", "Duplicate")],
        );

        let result = flow.validate();
        assert_eq!(result, Err(FlowValidationError::DuplicateStepId("step".to_string())));
    }
}
