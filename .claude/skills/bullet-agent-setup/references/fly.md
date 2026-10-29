# Fly.io notes

## Account and billing
- Sign up at `https://fly.io/app/sign-up`. A card is required. One shared-cpu-1x
  machine (512 MB) plus a 1 GB volume is roughly $5 a month in Tokyo (`nrt`).
- `fly auth login` opens a browser; the user signs in and returns. Never ask for
  tokens in chat.

## Billing check (right after `fly auth whoami`)
A trial org without a card fails only at the deploy ("This functionality is
disabled for trial organizations"). `fly orgs show --json` has no billing fields, so
ask Fly's API with the CLI's own token (read-only; `fly orgs list` gives the slug,
usually `personal`). The token reaches curl as a config on stdin (`-K -`), so it
never appears in a process list or on screen:
```sh
printf 'header = "Authorization: Bearer %s"\n' "$(fly auth token 2>/dev/null)" \
  | curl -s -K - https://api.fly.io/graphql -H 'Content-Type: application/json' \
    -d '{"query":"{ organization(slug:\"<org>\"){ isCreditCardSaved billingStatus } }"}'
```
Deployable: `isCreditCardSaved: true` (`billingStatus` `CURRENT`). `false`, or
`TRIAL_ACTIVE` / `TRIAL_ENDED` / `SOURCE_REQUIRED`: the user adds a card at
`https://fly.io/dashboard/<org>/billing`, then re-check. If the query fails, ask the
user to add a card at that link anyway before going on.

## Install
- macOS: `brew install flyctl`. Linux/WSL: `curl -L https://fly.io/install.sh | sh`,
  then call it as `~/.fly/bin/fly`. The commands here say `fly`; where only
  `flyctl` exists, write `flyctl` instead (same CLI, same arguments).

## What the deploy sets
This repo has no `fly.toml`: the Deploy workflow uses the framework's
(`deploy/agent/fly.toml` in `bulletxyz/bullet-bots`), so a merge can't change the
app's settings.
- One machine only: the trading key and history live on one volume (`data` at
  `/data`), and a volume attaches to a single machine. Never `fly scale count 2`.
- Public: `/mcp` and `/health` on port 3030 over HTTPS. Private only: `/status` and
  `/metrics` on 9091, scraped by Fly's managed Grafana (Fly dashboard, Metrics).
- `[env]` defaults: `BB_NETWORK=mainnet`, `BB_SYMBOL=BTC-USD`, `BB_MM_DRY_RUN=true`
  (paper). A Fly secret with the same name wins over `[env]`, across redeploys: that
  is how `BB_NETWORK=testnet` and going live (the workflow's `go-live` action sets
  the secret `BB_MM_DRY_RUN=false`) work. The truth about live or paper is the secret
  and the briefing, never a config file.
- Secrets the user owns (`fly secrets set`): `BB_MCP_PAIRING_CODE`,
  `BB_MM_MAX_POSITION`, `BB_MM_DAILY_LOSS_USD`, optional `BB_MM_STALE_SECS` (30 s
  mainnet, 60 s testnet by default), `BB_MAX_ORDERS_PER_MIN`, `BB_MAX_NOTIONAL_USD`,
  `BB_BULLET_ALLOW_MAIN_ACCOUNT=true` (main-account choice only),
  `BB_HOSTING_USD_PER_MONTH`, and the alerts variables. Leave
  `BB_BULLET_SUB_ACCOUNT` unset: the delegation picks the account. The MCP host name
  comes from Fly's `FLY_APP_NAME`; set `BB_PUBLIC_HOST` only for a custom domain.

## Everyday commands (always pass `--app <name>`)
- Logs: `fly logs --app <name>` (`--no-tail` for a snapshot)
- Status: `fly status --app <name>`
- Restart: `fly machine list --app <name>` for the machine `<id>`, then
  `fly machine restart <id> --app <name>`
- Change a limit (only with the user's explicit yes):
  `fly secrets set --app <name> BB_MM_MAX_POSITION=0.002` (restarts the agent; it
  adopts the venue's position in its account)
- Tear down: `fly apps destroy <name>`. Close the position first, and remove the
  delegate in the Bullet app. Destroying the volume destroys the agent's key.

## Deploys
`.github/workflows/deploy.yml` is the only thing that deploys; never `fly deploy` by
hand. It needs `fly-app: "<name>"` in that file and the repo secret `FLY_API_TOKEN`.
- Every merge to `master` builds and deploys the next version (`bot-vN` tag) by digest.
- Redeploy now: `gh workflow run deploy.yml`. Roll back:
  `gh workflow run deploy.yml -f version=<N>`. Live / paper:
  `-f action=go-live` / `-f action=paper` (paper only after pausing and, if wanted,
  closing the position while still live: SKILL.md step 10.6). Watch any of them with the run-id method
  in SKILL.md step 6 (record the newest run id first, then wait for a newer one).
- Renew the deploy token (expired or revoked; never through chat):
  `fly tokens create deploy --app <name> -x 8760h | gh secret set FLY_API_TOKEN --repo <repo>`.
