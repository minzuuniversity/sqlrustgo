use super::context::EvalContext;
use sqlrustgo_types::Value;

#[allow(dead_code)]
pub fn resolve_column(_name: &str, _ctx: &EvalContext) -> Value {
    Value::Null
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::trigger_eval::context::TriggerContext;

    #[test]
    fn resolve_column_returns_null_for_any_name() {
        let trigger = TriggerContext::new(None, None);
        let ctx = EvalContext::new(&trigger, None);
        // The stub returns Value::Null regardless of input.
        assert_eq!(resolve_column("foo", &ctx), Value::Null);
        assert_eq!(resolve_column("", &ctx), Value::Null);
        assert_eq!(resolve_column("any_column", &ctx), Value::Null);
    }
}
