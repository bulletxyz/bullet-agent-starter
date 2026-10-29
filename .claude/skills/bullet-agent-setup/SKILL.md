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
- `<repo>`: this repo as `owner/name`
  (`gh repo view --json nameWithOwner --jq .nameWithOwner`).
- `<name>`: the Fly app, `<owner>-<repo name>` (e.g. `alice-bullet-mm`), lowercased,
  with anything other than letters and digits turned into dashes. One repo and one
  Fly app per strategy, so the name says which strategy it runs. Taken: add `-2`,
  `-3`, ... Don't ask the user for a name.
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

Then ask which strategy to run, as a choice:
1. **Market maker**: quotes both sides of BTC-USD around the Binance price and
   earns the spread. This repo.
2. **Funding arb** (coming soon, not available yet): earns the funding-rate gap
   between Bullet and Hyperliquid, hedged.
Funding arb: say it isn't ready yet and offer the market maker for now; stop if
they don't want it.

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
   repo, create their copy, named after the strategy (taken: `bullet-mm-2`, ...):
   ```sh
   gh repo create bullet-mm --private --template bulletxyz/bullet-agent-starter --clone
   ```
   (While `bulletxyz/bullet-bots` is private, see step 5: create it in the org
   instead, `gh repo create bulletxyz/bullet-mm ...`, if they may.)
   Then tell them to open the folder it created (`bullet-mm`, or the suffixed name) in Claude Code and
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

### 3. Choose and fund the account
1. Read the order size from `agent.toml` (`order_size`: 0.0001 BTC in the starter,
   the venue minimum, about $8) and get the price
   (`curl -s "https://tradingapi.bullet.xyz/fapi/v1/ticker/price?symbol=BTC-USD"`).
   State the minimum to go live later: the agent quotes 2 orders of `order_size`,
   and its max position (at most 1x: equity / price) must cover both, so equity of
   at least 2 × `order_size` × price (about $16 at $80k with the starter's size; give
   the current figure). Paper trading needs no funds, so they can fund any time
   before going live.
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
   For a sub-account they create it in the app now and move in what they want the
   agent to trade with. Don't ask for its index or balance: they pick it when they
   add the delegate (step 7), and you read its balance from Bullet before going
   live (step 10).

### 4. The Fly app, its volume and secrets
1. Create the app and its volume in Tokyo (the deploy adds the machine):
   ```sh
   fly apps create <name> --org <org> && fly volumes create data --app <name> --region nrt --size 1 --yes
   ```
   Name taken: add the next suffix and retry.
2. Decide Telegram alerts (`references/telegram.md`): relay mode if the relay
   answers, otherwise their own bot (they run its command now), or none.
3. Make the sign-in code: run `openssl rand -hex 16` and note the 32-character code
   it prints. Then set every secret in ONE staged command (nothing runs yet, so
   nothing restarts):
   ```sh
   fly secrets set --app <name> --stage \
     BB_MCP_PAIRING_CODE=<code>
   ```
   No limits yet: paper trading caps itself at 10 orders' worth, and the limits are
   set from the account's balance before going live (step 10).
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
Then check whether the framework repo is public:
`gh repo view bulletxyz/bullet-bots --json visibility --jq .visibility`. Only
`PUBLIC` means nothing more is needed. `PRIVATE`, or any error (the user can't
read it, or the command failed): it is private, and the agent can't build without
these two things (an error: rerun it first; still an error, ask the Bullet team for
access and stop):
1. This repo must be in the `bulletxyz` org (only its repos may use the framework's
   deploy workflow). If `<repo>` is not `bulletxyz/...`, move it:
   `gh repo transfer <repo> bulletxyz` (needs permission to create repos in the
   org; otherwise ask the Bullet team), then `git remote set-url origin
   https://github.com/bulletxyz/<repo name>.git`, and use the new `<repo>` from
   here on (redo the `FLY_API_TOKEN` secret if it didn't move with the repo:
   `gh secret list --repo <repo>`).
2. The secret `BULLET_BOTS_TOKEN`: a fine-grained GitHub token with read-only
   **Contents** on `bulletxyz/bullet-bots`. Have the user create one at
   `https://github.com/settings/personal-access-tokens/new` (resource owner
   `bulletxyz`, only that repository, Contents: Read-only, and the **longest
   expiration** offered: the default is 30 days, and when it lapses CI and deploys
   fail with git authentication errors) and copy it. Never into chat. Then, on a
   Mac, have them run in this session
   `! pbpaste | gh secret set BULLET_BOTS_TOKEN --repo <repo>` (it reads the
   clipboard; then copy something else). Elsewhere, have them run
   `gh secret set BULLET_BOTS_TOKEN --repo <repo>` in a normal terminal and paste
   at its hidden prompt. Not `! gh secret set ...` without the pipe: a `!` command
   has no prompt, so it saves an EMPTY secret and the build still can't fetch the
   framework. Check `gh secret list --repo <repo>` shows it with the current time;
   if the first build fails with `failed to get bullet-agents`, the token is wrong
   or empty: set it again. To renew it later: a new token, the same command.

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
   Tell the user the build takes a few minutes (it starts from the release's
   prebuilt dependencies; on a framework release without them, the first build
   takes 15-25 minutes and later ones are faster). If your command times out, keep watching with
   `gh run watch <id> --repo <repo> --exit-status`. `deploy run did not start`: ask
   them to open `https://github.com/<repo>/actions` and tell you what they see, and
   stop. A failed run: `references/troubleshooting.md`, Deploy.
3. The deploy succeeds before delegation: the agent starts, paper trading, and waits
   for step 7 (`/health` answers `{"status":"waiting_for_delegation","address":...}`).

### 7. Delegate the agent's key on Bullet
1. Once a minute `fly logs --app <name> --no-tail` prints `waiting for delegation:
   add <address> as a delegate in the Bullet app (Delegate accounts)`, then `or open this link to
   add it: <link>` (address only: `curl -s https://<name>.fly.dev/health`).
2. Tell the user to open `<link>` (it prefills the agent's PUBLIC address, safe to
   share, and the name), pick the account in the form's account dropdown (the
   sub-account they funded, or the main account if they chose that), and approve in
   their wallet. If the form doesn't open prefilled:
   app.bullet.xyz (app.testnet.bullet.xyz on testnet), **Delegate accounts**, choose the account in the form's account
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
   - **Tool permissions** (Customize, Connectors, Bullet bot, Tool permissions):
     set the **read-only** tools group to **Always allow** in one go, and leave the
     rest (settings changes, pause, resume, close position) on "Needs approval", so
     nothing changes without a tap. Or simply tap "Always allow" the first time each
     read tool asks. (In Claude Cowork the option may be greyed out by the org
     admin: use a normal chat.)
   - **Telegram:** tap the link and press Start (own bot: send `/start <LINK>` to
     it, `references/telegram.md`).
   - **Try it:** a new chat with the connector on: "How's my agent doing?"

### 9. Improve it from the phone
Setup ends here, in this one session; nothing else needs a second session now. Tell
the user how tweaks work from now on (this local session stays the place for
deploys by hand, going live and framework updates):
- **Settings** (spread, size, skew, pause): ask in the Claude app with the connector
  on, e.g. "widen the spread a bit". Applied live, within bounds; the app asks before
  each change.
- **Code** (new behaviour): Claude Code on the web or in the Claude phone app, on
  this repo. Claude makes the change on a branch with tests and opens a pull request
  that says in plain words what changes and what to expect. Saying "ship it" (or
  tapping **Merge** in the GitHub app) deploys it as the next version.
Set the phone up now:
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
2. Read the account's balance from Bullet; don't ask for it. `<account>` is the
   `master=` address on the `Bullet: delegation found` line of `fly logs --app
   <name> --no-tail` (or `bullet_balances` in the connector). Its equity is
   `totalMarginBalance` in
   `curl -s "https://tradingapi.bullet.xyz/fapi/v3/account?address=<account>"`
   (a 404 means it is not funded yet), and get the price fresh now (step 3's ticker command; it may have moved since).
3. Propose the limits from that equity and wait for their yes:
   - `BB_MM_MAX_POSITION` in BTC = equity / BTC price (1x), rounded down to
     0.0001 BTC. Below 2 × the order size the agent runs now (`order_size` from
     `bullet_params`: they may have tuned it while paper trading) the account is
     too small: give the minimum for that size, and stop until they add funds or
     lower the size. Never raise the cap to make it fit.
     They can choose lower, or up to 3x if they ask (say first: at 3x a full position is liquidated by a ~30% move against it; the daily loss stop halts trading long before). Never above 3x.
   - `BB_MM_DAILY_LOSS_USD` = about 5% of equity.
   Explain: "max position caps the orders that would grow the position past that
   much BTC either way (orders that reduce it always go through); the daily loss is
   the amount at which it stops and cancels. They live on your Fly app, not in the
   code, so no code change can raise them." Restate the account-choice note from
   step 3. On their yes:
   `fly secrets set --app <name> BB_MM_MAX_POSITION=<btc> BB_MM_DAILY_LOSS_USD=<usd>`
   (restarts the agent, still paper).
4. Read `bullet_params` (the limits now in force) and `bullet_balances`, then ask for
   an explicit yes: "Go live with real orders on `<account>` ($<equity>), no new position-growing orders past <btc> BTC either way, stopping for the day at a $<usd> loss?"
5. Only after that yes, as one command:
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
6. Verify after the restart (`fly status --app <name>`): the next briefing does NOT
   say "paper trading" (or `fly ssh console --app <name> -C "printenv
   BB_MM_DRY_RUN"` prints `false`).
7. Back to paper at any time, in this order (in paper mode the agent can no longer
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
  log's link points there).
- Testnet price and balance: `tradingapi.testnet.bullet.xyz` in place of `tradingapi.bullet.xyz`, e.g.
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
