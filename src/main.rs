//! My Bullet agent: the framework's CLI with my market maker registered as
//! `simple-mm` (`[strategy] type = "simple-mm"` in `agent.toml`). The pricing
//! is mine (`mm.rs`, `fair_value.rs`); the framework runs it behind the hard
//! limits, guards and the risk gate.

mod fair_value;
mod mm;

use bullet_agents::Strategies;

fn main() -> std::process::ExitCode {
    bullet_agents::main(Strategies::new().market_maker("simple-mm", mm::SimpleMm::new))
}
