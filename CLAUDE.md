# My Bullet agent

This repo is one trading agent: the strategy is mine, the framework
(`bullet-agents`, a git dependency in `Cargo.toml`) runs everything else: the
CLI, exchange connections, hard limits, guards, the risk gate, alerts, `/status`
and the MCP connector.

- **The strategy is yours to change.** `src/mm.rs` (quotes) and
  `src/fair_value.rs` (fair value) implement `QuoteStrategy`; `agent.toml`
  configures it. Import framework types only from `bullet_agents::strategy` and
  the `bullet_agents::rust_decimal` / `bullet_agents::tracing` re-exports.
- **Never call the exchange outside the strategy API.** A strategy only returns
  quotes; the framework places them through the risk gate. Don't add exchange
  SDKs, HTTP clients or keys to this repo.
- **Hard limits live in Fly secrets**, never in this repo: `BB_MM_MAX_POSITION`,
  `BB_MM_DAILY_LOSS_USD`, `BB_MM_STALE_SECS`, `BB_MAX_ORDERS_PER_MIN`,
  `BB_MAX_NOTIONAL_USD`, `BB_BULLET_ALLOW_MAIN_ACCOUNT`. Putting any of them in
  `agent.toml` is a load error.
- **Never enable the `published-image` feature** of `bullet-agents`: it lets the
  config file set hard limits, and only the framework's own image may use it.
- Use `rust_decimal::Decimal` for prices and sizes, never floats; no panics
  (`unwrap`, `expect`, unchecked arithmetic) outside tests.
- Before committing: `cargo +nightly fmt`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`. Unit tests use `bullet_agents::testing` (a `Market` fixture for
  a `MarketView`, default params, books).
- **Skills:** first-time setup (Fly app, secrets, first deploy, connector) is
  `.claude/skills/bullet-agent-setup/`; everything after (how it's doing, changes,
  "ship it", rollback, go live, framework updates) is `.claude/skills/bullet-agent/`.
  A merge to `master` deploys: merge only when the user says "ship it".
