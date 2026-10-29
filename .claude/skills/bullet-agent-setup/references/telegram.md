# Telegram alerts

The agent sends alerts (fills, pauses, loss-limit stops, "vN is live") and answers a
few commands.
Pick ONE path. Both set Fly secrets; neither asks the user to look up a chat id.

## Relay mode (one tap, default when the relay is deployed)

Bullet's shared relay (if its `/healthz` does not answer `ok`, use the own-bot path):

    BB_ALERTS_RELAY_URL = https://bullet-agent.fly.dev
    BB_ALERTS_RELAY_BOT = bullet_agent   # bot username without @; optional

1. Append `BB_ALERTS_RELAY_URL=<url>` to the SKILL.md step-4 `fly secrets set`
   command (and `BB_ALERTS_RELAY_BOT=<username>` if known; it lets the log line be a
   full `t.me` link even if the relay does not return its name).
2. After the first deploy, the agent registers with the relay and logs ONCE:
   `Open this link in Telegram to receive alerts: https://t.me/<bot>?start=<code>`
   (or `Open the Bullet alert bot in Telegram and send: /start <code>`).
   Find it with `fly logs --app <name> --no-tail | grep -i telegram`.
3. User taps the link, presses Start. The bot confirms in chat.

Re-linking (new phone, lost chat): the link is shown only at first registration. Delete
the stored credentials and restart so it registers again and prints a fresh link:
`fly ssh console --app <name> -C "rm /data/alerts.json"`, then
`fly machine list --app <name>` for the machine `<id>` and
`fly machine restart <id> --app <name>`.

## Own bot (BotFather)

For users who want their own bot name.
1. In Telegram, message `@BotFather`, send `/newbot`, follow the prompts, copy the
   token (looks like `123456:ABC...`). The token is a secret: it never goes into
   chat or a command line. Ask the user to run this in their own terminal window,
   before the SKILL.md step-4 command (it waits for the paste without showing it,
   stages both secrets from stdin with `fly secrets import`, so the token is never
   in a process's arguments, and prints only the link code):
   ```sh
   read -rs TOKEN && LINK=$(openssl rand -hex 16) && printf 'BB_ALERTS_TELEGRAM_TOKEN=%s\nBB_ALERTS_LINK_CODE=%s\n' "$TOKEN" "$LINK" | fly secrets import --app <name> --stage && echo "link code: $LINK"
   ```
   They tell you the link code (`<LINK>`; at least 16 characters), not the token.
   This is the only place these two secrets are set: add nothing for Telegram to
   the step-4 command.
2. After the bot is up, the user opens their new bot in Telegram and sends
   `/start <LINK>` in a private chat with it (a group is refused, and `/status`,
   `/pause`, `/resume` only work in a private chat). The bot captures the chat id
   itself and saves it on the volume.
   (`BB_ALERTS_TELEGRAM_CHAT_ID` is only for users who already know their chat id.)

If both a token and a relay URL are set, the own bot wins.
Re-linking: delete `/data/alerts.json` as above, restart, send `/start <LINK>` again.
