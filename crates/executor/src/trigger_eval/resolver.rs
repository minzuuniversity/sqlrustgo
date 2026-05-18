use super::context::EvalContext;
use sqlrustgo_types::Value;

#[allow(dead_code)]
pub fn resolve_column(_name: &str, _ctx: &EvalContext) -> Value {
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger_eval::context::TriggerContext;

    #[test]
    fn test_resolve_column_returns_null() {
        let trigger_ctx = TriggerContext::new(None, None);
        let ctx = EvalContext::new(&trigger_ctx, None);
        let val = resolve_column("any_column", &ctx);
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn test_resolve_column_various_names() {
        let trigger_ctx = TriggerContext::new(None, None);
        let ctx = EvalContext::new(&trigger_ctx, None);
        assert_eq!(resolve_column("col1", &ctx), Value::Null);
        assert_eq!(resolve_column("a.b.c", &ctx), Value::Null);
        assert_eq!(resolve_column("", &ctx), Value::Null);
    }
}
