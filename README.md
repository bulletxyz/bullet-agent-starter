# My Bullet agent

A market-making agent for Bullet built on the `bullet-agents` framework. This
repo holds only the strategy (`src/mm.rs`, `src/fair_value.rs`) and its config
(`agent.toml`); the framework runs everything else, and hard limits live in
Fly secrets, never here.

You need a funded Bullet account, a GitHub account, a Fly.io account with a
card on file, and Claude Code.

1. Make your own private copy (skip this if you're reading this README in your own
   private agent repo already). With the GitHub CLI (`brew install gh` or
   https://cli.github.com, then `gh auth login --web`):
   `gh repo create bullet-mm --private --template bulletxyz/bullet-agent-starter --clone`
   Or on github.com: **Use this template**, private, then clone your new repo.
2. Open that folder in Claude Code (terminal or desktop) and say **set up my Bullet
   agent**. Claude runs the commands; you click links and approve
   (`.claude/skills/bullet-agent-setup/`). Your agent starts in paper trading.
3. Afterwards, from your phone: change settings in the Claude app with the agent's
   connector, and change the strategy with Claude Code on the web ("ship it" gives
   the go-ahead; merging the pull request, or tapping Merge in the GitHub app,
   deploys it; `.claude/skills/bullet-agent/`).
