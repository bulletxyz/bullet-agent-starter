---
name: bullet-agent-setup
description: Use when the user says "set up my Bullet agent", "deploy my agent", "set up a Bullet market maker" or "connect my agent to Claude / my phone" in this agent repo. Guides them, one step at a time, from a funded Bullet account to this repo's market maker running on their own Fly.io app in Tokyo, paper trading first, then tweaked from their phone.
---

# Set up my Bullet agent

This repo is the user's own agent: their strategy (`src/mm.rs`, `src/fair_value.rs`,
`agent.toml`) on the `bullet-agents` framework. Setup runs here, in local Claude Code
(terminal or desktop). The user has used Claude Code before but may never have used
GitHub, Fly.io or a terminal deploy. You run every command; they click links,
approve and answer questions. One step at a time, in plain words: say what you are
about to do and why, then do it. Afterwards they tweak the agent mostly from their
phone (step 9).

Hard rules:
- Never ask the user to paste a private key, seed phrase or Telegram token into chat
  (the one exception, the own-bot Telegram token, goes through a hidden terminal
  prompt: `references/telegram.md`).
- Never print the Fly deploy token or paste it into chat: it goes straight from `fly`
  into `gh secret set` (step 5).
- The agent makes its own key on the Fly machine. The user only pastes its PUBLIC
  address into Bullet.
- Do not place orders yourself, and do not call Bullet or the agent's API by hand
  beyond the reads below.
- Hard limits (`BB_MM_MAX_POSITION`, `BB_MM_DAILY_LOSS_USD`, ...) are Fly secrets
  only, set with the user's explicit yes. Never put them in `agent.toml` or any file.
- New agents paper trade. Real orders only in step 10, after an explicit yes.
- Don't change the strategy numbers in `agent.toml` during setup; the user tunes them
  later from the Claude app.

Names used below:
- `<handle>`: a short name the user picks (lowercase letters, digits, dashes).
- `<name>`: the Fly app, `<handle>-bullet-agent` (add a suffix if it is taken).
- `<repo>`: this repo as `owner/name`
  (`gh repo view --json nameWithOwner --jq .nameWithOwner`).
- `<org>`: the Fly organization slug (`fly orgs list`, usually `personal`).

Shell variables do not survive between your commands: run each block below as ONE
command, with the placeholders filled in.

## Steps

### 1. Say what this is
Say this plainly, in one sentence: this is software that trades real money, you can
lose money and the losses are yours, Bullet's terms apply, so start small. Then say
what setup does: about 20 minutes of their time (plus a build that runs on its own),
and they need a GitHub account, a Fly.io account with a card (about $5 a month) and
a Bullet account they can fund.

### 2. GitHub and Fly on this computer
1. GitHub CLI: `command -v gh && gh auth status`. Missing: macOS `brew install gh`,
   Linux/WSL `https://github.com/cli/cli#installation`. Not logged in (no account
   yet: free at `https://github.com/signup`): `gh auth login --web` and wait for
   them to finish in the browser. Pushing the deploy setting needs the `workflow`
   scope: if `gh auth status` does not list it, `gh auth refresh -h github.com -s
   workflow` (browser again). Then `gh auth setup-git`.
2. Check this is the user's own private copy:
   `gh repo view --json nameWithOwner,visibility,isTemplate`. If it is
   `bulletxyz/bullet-agent-starter` itself, a template, not `PRIVATE`, or there is no
   repo, create their copy (ask for `<handle>` first):
   ```sh
   gh repo create <handle>-bullet-agent --private --template bulletxyz/bullet-agent-starter --clone
   ```
   Then tell them to open the new `<handle>-bullet-agent` folder in Claude Code and
   say "set up my Bullet agent" again, and stop here.
3. If `git config user.email` prints nothing, set a repo-local author:
   `git config user.name "<owner>"` and
   `git config user.email "<id>+<owner>@users.noreply.github.com"`
   (`<owner>` and `<id>` from `gh api user --jq '.login, .id'`; Claude Code asks
   before any `gh api` call, so tell them it is a read).
4. Fly CLI: `command -v fly || command -v flyctl`. Missing: macOS
   `brew install flyctl` (installs both names), otherwise
   `curl -L https://fly.io/install.sh | sh`. Every command in this skill and its
   references says `fly`: if only `flyctl` is found, write `flyctl` in its place
   everywhere; after the install script, `~/.fly/bin/fly` (shell aliases don't
   survive between your commands). `fly auth whoami`; not logged in: tell them Fly needs an
   account with a card (about $5 a month for this agent), then `fly auth login`
   (browser; wait for them). Details: `references/fly.md`.
5. Billing, before creating anything (a trial org without a card only fails at the
   deploy): run the check in `references/fly.md` ("Billing check"). If
   `isCreditCardSaved` is not `true`, send them to
   `https://fly.io/dashboard/<org>/billing` to add a card, then check again.

### 3. Fund the account and propose limits
1. Read the order size from `agent.toml` (`order_size`, 0.0003 BTC in the starter)
   and get the price
   (`curl -s "https://tradingapi.bullet.xyz/fapi/v1/ticker/price?symbol=BTC-USD"`).
   State the minimum before they fund: the agent quotes 2 orders of `order_size`, so
   its max position (20% of equity) must be at least 2 × `order_size`, i.e. equity
   of at least 2 × `order_size` × price × 5 (0.0006 × price × 5 with the starter's
   size: about $245 at $81.5k; give the current figure).
2. In the same message, offer the account choice:
   - **Sub-account (recommended).** The agent's key is a delegate on that sub-account
     only, so Bullet itself stops it touching the main account or other sub-accounts.
     Opening a sub-account costs about $1.
   - **Main account.** No sub-account needed, but the agent treats the account's
     whole BTC-USD state as its own: every 60 s it cancels BTC-USD orders it didn't
     place, it counts any BTC-USD position toward its caps and quotes to unwind it,
     and its close tool would flatten the whole position. The key can trade
     everything else in the account too (it can never withdraw).
   No funded Bullet account yet: `https://app.bullet.xyz/trade?r=BB` (referral `BB`).
   Have them fund the chosen account (for a sub-account: create it, move the capital
   in) and reply with its **index** (the number next to it in the app: "Subaccount 1"
   is 1) and its **equity in USD**.
3. Propose the limits and wait for their yes before using them:
   - `BB_MM_MAX_POSITION` in BTC = about 20% of equity / BTC price (round down to
     0.0001 BTC steps). Below 2 × `order_size` the account is too small: stop until
     they add funds. Never raise the cap above 20% of equity to make it fit.
   - `BB_MM_DAILY_LOSS_USD` = about 5% of equity.
   Explain: "max position caps the orders that would grow the position past that
   much BTC either way (orders that reduce it always go through); the daily
   loss is the amount at which it stops and cancels. They live on your Fly app, not
   in the code, so no code change can raise them."

### 4. The Fly app, its volume and secrets
1. Ask for `<handle>` if you don't have it yet. Create the app and its volume in
   Tokyo (the deploy adds the machine):
   ```sh
   fly apps create <name> --org <org> && fly volumes create data --app <name> --region nrt --size 1 --yes
   ```
   Name taken: add a suffix and retry.
2. Decide Telegram alerts (`references/telegram.md`): relay mode if the relay
   answers, otherwise their own bot (they run its command now), or none.
3. Make the sign-in code: run `openssl rand -hex 16` and note the 32-character code
   it prints. Then set every secret in ONE staged command (nothing runs yet, so
   nothing restarts):
   ```sh
   fly secrets set --app <name> --stage \
     BB_MCP_PAIRING_CODE=<code> \
     BB_MM_MAX_POSITION=<btc> \
     BB_MM_DAILY_LOSS_USD=<usd>
   ```
   Append to this same command:
   - relay mode: `BB_ALERTS_RELAY_URL=...` (`references/telegram.md`). Own bot:
     nothing here; the user already staged its two secrets with the telegram.md
     command, which keeps the token out of chat and command lines;
   - `BB_NETWORK=testnet` only for a testnet rehearsal (step 11), and
     `BB_SYMBOL=<symbol>` only if they want a market other than BTC-USD (re-do the
     step-3 maths with that market's price first);
   - main-account choice only: `BB_BULLET_ALLOW_MAIN_ACCOUNT=true`.
   Never set `BB_BULLET_SUB_ACCOUNT`: the delegation itself picks the account. The
   stale-price guard is preconfigured (30 s mainnet, 60 s testnet); add
   `BB_MM_STALE_SECS=<secs>` only to override it. Give the pairing code to the user
   only in step 8.

### 5. Let this repo deploy to the app
The token goes straight from Fly into the repo's secret; never print it:
```sh
fly tokens create deploy --app <name> -x 8760h | gh secret set FLY_API_TOKEN --repo <repo>
```

### 6. First deploy (paper)
Every merge to `master` deploys the next version (v1, v2, ...) through
`.github/workflows/deploy.yml`, which does nothing until it names the app.
1. Edit `.github/workflows/deploy.yml`: change `fly-app: ""` to `fly-app: "<name>"`.
   Change nothing else in that file.
2. Commit, push and watch, as one command. It records the newest Deploy run before
   the push, then waits (about 2 minutes at most) for a newer one and watches it:
   ```sh
   before=$(gh run list --repo <repo> --workflow deploy.yml --limit 10 --json databaseId --jq 'map(.databaseId) | max // 0')
   git add .github/workflows/deploy.yml && git commit -m "chore: deploy to <name>" && git push origin master || { echo "could not push"; exit 1; }
   id=; for i in $(seq 40); do
     n=$(gh run list --repo <repo> --workflow deploy.yml --limit 10 --json databaseId --jq 'map(.databaseId) | max // 0')
     if [ "$n" -gt "$before" ]; then id=$n; break; fi; sleep 3
   done
   if [ -n "$id" ]; then echo "run $id"; gh run watch "$id" --repo <repo> --exit-status; else echo "deploy run did not start"; false; fi
   ```
   Tell the user the first build compiles the whole agent and takes a while (15-25
   minutes; later changes are faster). If your command times out, keep watching with
   `gh run watch <id> --repo <repo> --exit-status`. `deploy run did not start`: ask
   them to open `https://github.com/<repo>/actions` and tell you what they see, and
   stop. A failed run: `references/troubleshooting.md`, Deploy.
3. The deploy succeeds before delegation: the agent starts, paper trading, and waits
   for step 7 (`/health` answers `{"status":"waiting_for_delegation","address":...}`).

### 7. Delegate the agent's key on Bullet
1. Once a minute `fly logs --app <name> --no-tail` prints `waiting for delegation:
   add <address> at app.bullet.xyz → Delegate accounts`, then `or open this link to
   add it: <link>` (address only: `curl -s https://<name>.fly.dev/health`).
2. Sub-account: append `&subAccount=<i>` (the index from step 3) to `<link>`, so the
   form opens on the agent's sub-account. Main account: use `<link>` as is. Tell the
   user to open it (it prefills the agent's PUBLIC address, safe to share, and the
   name) and approve in their wallet. If the form doesn't open prefilled:
   app.bullet.xyz, **Delegate accounts**, choose the account in the form's account
   dropdown, add a delegate named "market maker", paste `<address>`.
3. Watch `fly logs --app <name>` until `Bullet: delegation found`. On a sub-account,
   `master=` and the `connected + subscribed` line's `account=` are the
   sub-account's own address with `sub_account=None`: correct. `this delegate is on
   your main account`: re-add it on the sub-account, or (main-account choice only)
   the `BB_BULLET_ALLOW_MAIN_ACCOUNT=true` secret is missing. Nothing after 3
   minutes: `references/troubleshooting.md`, Delegation.

### 8. Connect the Claude app (and Telegram)
1. Find `Add this bot to Claude (one tap): <link>` in `fly logs --app <name>
   --no-tail` (or `fly ssh console --app <name> -C "bb-bot mcp link"`). Relay mode:
   also `Open this link in Telegram to receive alerts: https://t.me/<bot>?start=<code>`.
2. On a Mac, `printf %s <pairing code> | pbcopy` and say it is copied.
3. Send in one message:
   - **Claude app** (web, desktop or phone): tap the one-tap link and add it. Sign
     in with **Sign in with wallet** (shown once the agent knows their Bullet
     wallet) or the pairing code `<code>` (always works; shown only here, so save it
     in a password manager). Fallback: Settings, Connectors, Add custom connector,
     name "Bullet bot", URL `https://<name>.fly.dev/mcp`.
   - **Tool permissions** (Settings, Connectors, Bullet bot): **Always allow** the
     read tools `bullet_briefing`, `bullet_positions`, `bullet_balances`,
     `bullet_params`, `bullet_history`, `bullet_logs`, `bullet_metrics`,
     `bullet_pnl_attribution`, `bullet_chart`, `bullet_funding_opportunities`,
     `bullet_versions`; leave `bullet_set_params`, `bullet_pause`, `bullet_resume`,
     `bullet_close_position` on "Needs approval". (In Claude Cowork the option may
     be greyed out by the org admin: use a normal chat.)
   - **Telegram:** tap the link and press Start (own bot: send `/start <LINK>` to
     it, `references/telegram.md`).
   - **Try it:** a new chat with the connector on: "How's my agent doing?"

### 9. Improve it from the phone
Tell the user how tweaks work from now on:
- **Settings** (spread, size, skew, pause): ask in the Claude app with the connector
  on, e.g. "widen the spread a bit". Applied live, within bounds; the app asks before
  each change.
- **Code** (new behaviour): Claude Code on the web or in the Claude phone app, on
  this repo. Claude makes the change on a branch with tests and opens a pull request
  that says in plain words what changes and what to expect. Saying "ship it" (or
  tapping **Merge** in the GitHub app) deploys it as the next version.
Set it up now:
1. Give Claude Code on the web access to this private repo: install the Claude
   GitHub App on it (`https://github.com/apps/claude`, choose "Only select
   repositories", pick `<repo>`), or run `/web-setup` once in this Claude Code.
2. Give them their **Improve my agent** link, with `<repo>` filled in, and ask them
   to open it on their phone and bookmark it (or add it to the home screen):
   `https://claude.ai/code?repositories=<repo>&prompt=How%27s%20my%20agent%20doing%3F`
3. Suggest the GitHub app on their phone (to tap Merge) and Telegram for alerts.

### 10. Paper first, then going live (an explicit yes)
The agent paper trades: it works out its quotes but sends no real orders, so nothing
is at risk. Hosting is about $5 a month. Ask them to come back after 4+ hours and ask
the connector "what would my agent have done?" and "why no fills?" (would-be fills
and 5s/60s markouts; negative means picked off; paper fills ignore queue position, so
they are optimistic), and to tune spread, size and skew in the Claude app first.

Going live, only when they come back and ask for it; never on your own:
1. Say this plainly, in one sentence: from now on this trades real money, you can
   lose money and the losses are yours, Bullet's terms apply, so start small.
2. Restate in plain words: the agent's account and current balance (the Bullet app,
   or `bullet_balances`), max position `BB_MM_MAX_POSITION` = <btc> BTC and daily
   loss `BB_MM_DAILY_LOSS_USD` = $<usd> (`fly secrets list --app <name>` shows they
   are set, not their values: use the ones agreed in step 3), and the account-choice
   note from step 3. Offer to change the limits first
   (`fly secrets set --app <name> BB_MM_MAX_POSITION=<btc>`; restarts the agent).
3. Ask for an explicit yes: "Go live with real orders on `<account>`, no new
   position-growing orders past <btc> BTC either way, stopping for the day at a $<usd> loss?"
4. Only after that yes, as one command:
   ```sh
   before=$(gh run list --repo <repo> --workflow deploy.yml --limit 10 --json databaseId --jq 'map(.databaseId) | max // 0')
   gh workflow run deploy.yml --repo <repo> -f action=go-live || { echo "could not start go-live"; exit 1; }
   id=; for i in $(seq 40); do
     n=$(gh run list --repo <repo> --workflow deploy.yml --limit 10 --json databaseId --jq 'map(.databaseId) | max // 0')
     if [ "$n" -gt "$before" ]; then id=$n; break; fi; sleep 3
   done
   if [ -n "$id" ]; then gh run watch "$id" --repo <repo> --exit-status; else echo "go-live run did not start"; false; fi
   ```
   The workflow refuses (still paper) if either limit secret is missing.
5. Verify after the restart (`fly status --app <name>`): the next briefing does NOT
   say "paper trading" (or `fly ssh console --app <name> -C "printenv
   BB_MM_DRY_RUN"` prints `false`).
6. Back to paper at any time, in this order (in paper mode the agent can no longer
   cancel real orders or close a real position, so do both while it is live):
   `bullet_pause` (cancels the resting orders); ask whether to close the position
   and only on a yes `bullet_close_position`; check `bullet_positions` and
   `bullet_briefing` (paused, no quotes out, the position they chose); only then
   the same command with `-f action=paper` (or Actions, Deploy, Run workflow,
   action `paper`).

Hazard: Fly secrets take precedence over `[env]` with the same name, across
redeploys. The framework's Fly config says `BB_MM_DRY_RUN = "true"`; after going live
the secret says false and wins. The truth about live or paper is the secret and the
briefing, never a config file. Switch only through the workflow's `go-live` / `paper`
actions.

### 11. Rehearse on testnet (optional, before mainnet)
The same flow with `BB_NETWORK=testnet` in the step-4 secrets (a secret, since it
must override the framework's `[env]`); the stale-price default is 60 s there.
- Use `https://app.testnet.bullet.xyz` for the faucet and Delegate accounts (the
  log's link points there; its `add <address> at app.bullet.xyz` line does not).
- Size limits from the testnet price:
  `https://tradingapi.testnet.bullet.xyz/fapi/v1/ticker/price?symbol=BTC-USD`.
- Moving to mainnet later: `fly secrets unset --app <name> BB_NETWORK`, then redo
  steps 3 and 7 for the mainnet account.

## After setup
The `bullet-agent` skill (`.claude/skills/bullet-agent/SKILL.md`) handles everything
after this: how it's doing, changes, ship it, rollback, go live, framework updates.

## References
- `references/fly.md`: Fly account, billing check, what the deploy sets, everyday
  commands, deploy token, tearing down.
- `references/telegram.md`: relay and own-bot paths, re-linking.
- `references/troubleshooting.md`: deploy, delegation, trading, connector.
