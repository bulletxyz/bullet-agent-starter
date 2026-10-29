---
name: bullet-agent
description: Use for anything about this running Bullet agent after setup - "how's my agent doing?", "am I getting picked off?", "widen when Binance moves fast" or any strategy change, "ship it", "what changed in v14?", "roll back to v12", "go live", "back to paper", "update the framework", "get the blueprint improvements". Works in local Claude Code and in Claude Code on the web or phone.
---

# My Bullet agent

The user talks; you do the git. They may never type a git command or open GitHub,
apart from tapping **Merge** in the GitHub app. Plain words, short answers, and say
what you are about to do before you do it. Setup is the `bullet-agent-setup` skill.

Where things are:
- The strategy: `src/mm.rs` (quotes) and `src/fair_value.rs` (fair value);
  `agent.toml` configures it. The framework (`bullet-agents`, pinned in `Cargo.toml`)
  runs everything else.
- The Fly app: `fly-app:` in `.github/workflows/deploy.yml` (`<name>` below).
  `<repo>` is this repo as `owner/name`
  (`gh repo view --json nameWithOwner --jq .nameWithOwner`).
- Every merge to `master` deploys the next version, vN (git tag `bot-vN`).

Where you are: `CLAUDE_CODE_REMOTE=true` in the environment means Claude Code on the
web or phone. There `gh pr merge` fails (GitHub's GraphQL API is blocked), `gh` may
be missing, and Fly and Bullet are out of reach; you don't need them. Live data comes
through the connector, deploys run in GitHub Actions, and the user's tap in the
GitHub app is the approval. Locally, `gh` and `fly` work.

Never:
- call the exchange (or add an exchange SDK, HTTP client or key) outside the
  strategy API: a strategy returns quotes, and the framework places them through
  the risk gate;
- touch a hard limit except as a Fly secret, from local Claude Code, after the
  user's explicit yes (below); never in `agent.toml` or any file in this repo;
- enable the framework's `published-image` feature;
- merge, go live or change a limit without the user saying so in this conversation.

## "How's it doing?"
Use the connector's tools (the "Bullet bot" connector): `bullet_briefing` first,
then as the question needs: `bullet_pnl_attribution` (where P&L comes from),
`bullet_metrics` (markouts: negative means picked off), `bullet_history`,
`bullet_positions`, `bullet_versions` (results per version), `bullet_logs`. Read the
code alongside when the question is about why. Answer in a few plain sentences with
the numbers that matter.

No connector tools in this session: locally, add it with
`claude mcp add --transport http bullet-bot https://<name>.fly.dev/mcp` and sign in
through `/mcp` (wallet or the pairing code); on the web, turn on the Bullet bot
connector for the session. Don't scrape the agent's or the exchange's API instead.

## "Widen the spread", "smaller size" (a setting)
Settings the strategy already has change live, without code: `bullet_set_params`
(the user approves in the app; values stay within bounds). Use this whenever it
covers the request.

## "Change X" (new behaviour: code)
1. Agree the change in a sentence or two, and what it should do to the numbers.
2. Branch from an up-to-date `master`
   (`git switch master && git pull --ff-only && git switch -c <short-name>`), edit
   `src/mm.rs` / `src/fair_value.rs`. Import framework types only from
   `bullet_agents::strategy` (and `bullet_agents::rust_decimal`,
   `bullet_agents::tracing`). `Decimal` for prices and sizes; no panics.
3. A new knob: declare it in the strategy's `extra_params()`
   (`ExtraParam { name, default, min, max, help }`, tight sensible bounds) and read it
   with `view.params.get("<name>")`. It becomes tunable live through
   `bullet_set_params`'s `extra`; set a starting value in agent.toml's
   `[strategy.simple-mm.extra]` table if the default isn't right.
4. Tests next to the change, with `bullet_agents::testing` (a `Market` fixture for a
   `MarketView`, default params, books), showing the new behaviour.
5. `cargo +nightly fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
   (where cargo is available; CI runs them on the pull request either way).
6. Commit, push the branch, open a pull request (`gh pr create`; on the web, if that
   fails, the session's create-PR button or
   `https://github.com/<repo>/compare/master...<branch>?expand=1`). Title in plain
   words. The description, for someone who doesn't read code:
   - **What changes:** one or two sentences.
   - **Expected effect:** on fills, spread, markouts or P&L, and the risk.
   - **How to check:** what to ask after it has run a few hours ("did my change
     work?").
7. Stop at "ready to ship" with the link. Don't merge.

## "Ship it"
The user's go-ahead for the pull request you just discussed. Merging deploys it.
- **Local:** `gh pr merge <number> --squash --delete-branch`. Claude Code asks the
  user to approve the command: that tap is their sign-off.
- **Web or phone:** don't try `gh pr merge`. Give the pull request link and say:
  "tap **Merge** (squash) in the GitHub app; that deploys it as the next version."
Then the deploy runs in Actions (`https://github.com/<repo>/actions`). Locally you
can watch it with the run-id method: record the newest Deploy run id before
merging, then wait for a newer one and `gh run watch <id> --exit-status`. When it is
live, Telegram says "vN is live" and `bullet_versions` lists it.

## "What changed in v14? Is v15 better?"
`bullet_versions` for each version's results, and the merge behind it
(`git log -1 bot-v14`, or the pull request) for what changed.

## "Roll back to vN"
`gh workflow run deploy.yml -f version=<N>` redeploys version N's image as it was
built. No `gh` here (web): give the user
`https://github.com/<repo>/actions/workflows/deploy.yml`, **Run workflow**, version
`<N>`. A rollback changes what runs, not the code on `master`: the next merge deploys
`master` again, so to keep the old behaviour also revert the change in a pull request.

## "Go live" / "back to paper"
- **Go live** (real orders): only when the user asks. Say plainly, in one sentence:
  from now on this trades real money, you can lose money and the losses are yours,
  Bullet's terms apply, so start small. Read the limits with `bullet_params` and
  the balance with `bullet_balances`, then restate the account, its balance, the
  max position and the daily loss, with their values. Ask: "Go live with real orders, no
  position-growing orders past <max position> either way, stopping for the day at a <daily loss> loss?" Only after a clear yes:
  `gh workflow run deploy.yml -f action=go-live` (web: Actions, Deploy, Run
  workflow, action `go-live`). The workflow refuses if the limit secrets are
  missing. Check the next briefing no longer says "paper trading".
- **Back to paper**, in this order (in paper mode the agent can no longer cancel
  real orders or close a real position, so do both while it is still live):
  1. `bullet_pause`: cancels the resting orders and stops quoting (its reply says
     whether the cancel went through).
  2. Ask whether to close the position; only on a yes, `bullet_close_position`.
  3. Check: `bullet_positions` shows the position they chose (flat, or kept), and
     `bullet_briefing` shows the agent paused with no quotes out. If not, stop and
     sort it out before going on.
  4. Then `gh workflow run deploy.yml -f action=paper` (or the same Actions route,
     action `paper`).

## "Change my limits"
Only from local Claude Code (the web can't reach Fly), and only after the user's
explicit yes to the new values: `fly secrets set --app <name>
BB_MM_MAX_POSITION=<btc>` (or `BB_MM_DAILY_LOSS_USD=<usd>`). It restarts the agent.
Keep max position at or under about 20% of equity and daily loss about 5%, unless
the user insists after hearing why. From the phone, `bullet_set_params` can tighten
within the limits; it can never raise them.

## "Update the framework"
A version bump, shipped like any change:
1. Newest release: `gh release list --repo bulletxyz/bullet-bots --limit 5`; what's
   in it: `gh release view v<M> --repo bulletxyz/bullet-bots`.
2. In one branch, change all three together to the same `v<M>`: both
   `bullet-agents` entries in `Cargo.toml` (`[dependencies]` and
   `[dev-dependencies]`, `tag = "v<M>"`) and the `@v<N>` at the end of the `uses:`
   line in `.github/workflows/deploy.yml`. Then `cargo update -p bullet-agents` and
   `cargo test`; fix anything the new release changed.
3. A pull request saying in plain words what the new release brings; "ship it" as
   usual.

## "Get the blueprint improvements"
The strategy started as a copy of Bullet's market maker blueprint and is the user's
now. Never overwrite it with the new blueprint.
1. `<from>`: the framework tag the strategy was last synced with (the `Cargo.toml`
   tag before any bump in this conversation); `<to>`: the newest release.
2. Diff the blueprint between them:
   ```sh
   git clone --quiet --filter=blob:none https://github.com/bulletxyz/bullet-bots /tmp/bullet-bots-blueprint && git -C /tmp/bullet-bots-blueprint diff v<from> v<to> -- crates/strategies/simple-mm/src/blueprint.rs crates/strategies/simple-mm/src/fair_value.rs starter/
   ```
   (`blueprint.rs` is the upstream of `src/mm.rs`; `fair_value.rs` of
   `src/fair_value.rs`; `starter/` holds `agent.toml`, `CLAUDE.md` and these
   skills.)
3. Explain each improvement in a sentence and ask which they want. Apply only those,
   by hand, onto their code (it may differ from the blueprint), with tests, in a pull
   request; "ship it" as usual.
