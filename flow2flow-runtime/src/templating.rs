use minijinja::Environment;
use serde_json::Value;

use crate::ctx::Ctx;

#[derive(Debug)]
pub struct TemplateEngine {
    env: Environment<'static>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        let env = Environment::new();
        Self { env }
    }
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render_value(&self, template: &str, ctx: &Ctx) -> Result<Value, minijinja::Error> {
        let snapshot = ctx.template_snapshot();
        self.render_with_data(template, &snapshot)
    }

    pub fn render_with_data(
        &self,
        template: &str,
        data: &Value,
    ) -> Result<Value, minijinja::Error> {
        let rendered = self.env.render_str(template, data)?;
        match serde_json::from_str::<Value>(&rendered) {
            Ok(value) => Ok(value),
            Err(_) => Ok(Value::String(rendered)),
        }
    }
}
