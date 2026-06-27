use crate::gate;

pub fn run_gate(stage: &str, mode: &str) -> anyhow::Result<()> {
    match stage {
        "alpha" => gate::alpha::run(),
        "beta" => gate::beta::run(mode),
        "rc" => gate::rc::run(),
        "ga" => gate::ga::run(),
        _ => anyhow::bail!("unknown gate stage: {stage} (valid: alpha | beta | rc | ga)"),
    }
}
