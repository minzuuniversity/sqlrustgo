use crate::gate;

pub fn run_gate(stage: &str) -> anyhow::Result<()> {
    match stage {
        "alpha" => gate::alpha::run()?,
        "beta"  => gate::beta::run()?,
        "rc"    => gate::rc::run()?,
        _ => anyhow::bail!("unknown gate stage: {stage} (valid: alpha | beta | rc)"),
    }
    Ok(())
}