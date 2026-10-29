# Troubleshooting

Always start with `fly logs --app <name> --no-tail` and `fly status --app <name>`.

## Deploy
`<repo>` is this repo as `owner/name`; `<name>` the Fly app (SKILL.md, names).
- **`gh repo create` fails on the template**: `bulletxyz/bullet-agent-starter` is not
  available as a template. Ask the Bullet team.
- **Push refused: `refusing to allow an OAuth App to create or update workflow`**: the
  GitHub login lacks the `workflow` scope:
  `gh auth refresh -h github.com -s workflow`, `gh auth setup-git`, push again.
- **Deploy run skipped / nothing deployed**: `.github/workflows/deploy.yml` still has
  `fly-app: ""`, or the run was not on `master`. Set the app name (SKILL.md step 6) and
  push to `master`.
- **`the FLY_API_TOKEN secret is not set`, or the Fly step says unauthorized**:
  renew the token, piped, never printed:
  `fly tokens create deploy --app <name> -x 8760h | gh secret set FLY_API_TOKEN --repo <repo>`,
  then `gh run rerun <run-id> --repo <repo> --failed`.
- **Build fails compiling the strategy**: the change broke the build; CI on the pull
  request shows the same error. Fix it in a new pull request, or roll back.
- **Other failed run**: `gh run view <run-id> --repo <repo> --log-failed`.
- **Rollback says `no version vN in this repo`**: that number was never built; pick
  one from `bullet_versions` (its 20 newest), or for older ones the repo's tags:
  `git fetch --tags && git tag -l 'bot-v*'`. Nothing was deployed.
- **go-live says `has no BB_MM_MAX_POSITION secret` (or `BB_MM_DAILY_LOSS_USD`)**: the
  limits are not set; still paper trading. Set them with the user's yes (SKILL.md
  step 10), then run go-live again.
- **App name taken**: add a suffix and redo `fly apps create`; then the volume, the
  secrets, the token and `fly-app` all use the new name.
- **Volume error / machine won't start**: the volume must exist in `nrt` before the
  deploy: `fly volumes list --app <name>`.
- **`This functionality is disabled for trial organizations`**: no card on the Fly
  org. Add one at `https://fly.io/dashboard/<org>/billing`, then
  `gh run rerun <run-id> --repo <repo> --failed`.
- **Deploy says unhealthy**: `/health` answers even while waiting for the
  delegation, so this is not the delegation. The first boot can take up to 30 s
  (health `grace_period`). If the log shows `MCP not served: BB_MCP_PAIRING_CODE`,
  the pairing code secret is missing or shorter than 16 characters.

## Delegation
- No `waiting for delegation` line: while the bot waits for its delegation,
  `curl -s https://<name>.fly.dev/health` shows the address. Otherwise find the bare
  address printed at first boot (scroll up), or run
  `fly ssh console --app <name> -C "bb-bot keygen --key-file /data/bullet.key --if-missing"`
  (prints the existing address; never overwrites).
- Added the delegate but no `Bullet: delegation found` after 3 minutes: the address
  was mistyped or the delegation was added from another Bullet account than the funded
  one. The bot polls every few seconds; check the address matches exactly.
- `this key is already scoped to a sub-account; unset BB_BULLET_SUB_ACCOUNT`:
  `fly secrets unset --app <name> BB_BULLET_SUB_ACCOUNT`; the delegation picks the
  account.
- `this delegate is on your main account; create it on the bot's sub-account`: the
  delegate was added on the main account, so the bot refuses to start (it would
  otherwise cancel the user's BTC-USD orders there and could close their position).
  If they chose a sub-account: remove the delegate and add it again in Bullet,
  Delegate accounts, with the sub-account picked. Set
  `BB_BULLET_ALLOW_MAIN_ACCOUNT=true` only if they chose the main account in step 3
  after hearing its caveats; never as a quick fix.
- `could not check that the delegate is on a sub-account ... refusing to start`: the
  account lookup failed (venue API blip). The machine restarts on its own; if it
  persists, check `fly logs` for the HTTP error.

## Trading
- **No real orders / fills**: the bot is probably still paper trading. The briefing
  says so. Going live is step 10 of `SKILL.md`; back to paper is step 10.6 (pause
  and, if wanted, close the position while still live, then `-f action=paper`).
- `user_variants not found` or order rejections: the bot's account has no balance.
- Bot stopped by itself: the daily loss or max position limit tripped, or the data
  feed was stale longer than `BB_MM_STALE_SECS`. `bullet_briefing` says which.
  Resume with `bullet_resume` after checking.

## Connector
- Sign-in rejects the code: it must be the exact `BB_MCP_PAIRING_CODE`. Make a new one
  with `fly ssh console --app <name> -C "bb-bot mcp rotate-pairing"`, then
  `fly secrets set --app <name> BB_MCP_PAIRING_CODE=<code>`.
- Connector says it cannot reach the server: open `https://<name>.fly.dev/health`; if
  that fails the machine is down (`fly status`).
- "Always allow" greyed out in Cowork: org admin policy; use a normal chat.
- Tools ask for approval every time: Tool permissions were not saved; set them again.
- **Grok Bot**: its OAuth callback may not be on the bot's allowlist yet (known gap),
  so sign-in can fail with a redirect error. Tell the user honestly; use the Claude
  app until it is added.
- Sign everyone out: `fly ssh console --app <name> -C "bb-bot mcp revoke-all"`.

## Telegram
- No link line in logs: relay unreachable (`alert relay registration failed`) or not
  configured (`alerts off: set BB_ALERTS_RELAY_URL ...`). Link code must be 16+ chars.
- Link line already scrolled away: see re-linking in `telegram.md`.

## Claude Code on the web (phone)
- The session can't open or push to the repo: Claude has no access to it. Install
  the Claude GitHub App on the repo (`https://github.com/apps/claude`), or run
  `/web-setup` once in local Claude Code.
- `gh pr merge` (or `gh` at all) fails in a web session: expected. Give the user the
  pull request link; they tap **Merge** in the GitHub app.
- The web session can't reach Fly or Bullet: expected, and not needed. Deploys run
  in GitHub Actions, live data comes through the connector, and limits change only
  from local Claude Code.
