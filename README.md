[![CI](https://github.com/exla-ai/cuz/actions/workflows/ci.yml/badge.svg)](https://github.com/exla-ai/cuz/actions/workflows/ci.yml)

# cuz

Give every piece of code a traceable reason for existing.

## Get started

```sh
brew tap exla-ai/cuz
brew install cuz
```

That's it. Every Claude Code session will now automatically track *why* changes are made — not just what changed. No config, no setup, no per-repo init needed.

## What happens

After install, Claude Code will:

1. Run `cuz init` in any repo it works in (auto, first commit only)
2. Create a `.cuz/intents/cuz_XXXXXX.json` for every commit — recording the goal, approach, alternatives considered, and confidence
3. Add an `Intent: cuz_XXXXXX` trailer to each commit message
4. Read existing intents before modifying code, so it knows why things are the way they are

## Query it

```sh
cuz why src/retry.ts:42       # why does this line exist?
cuz log                        # intent history
cuz search "backoff"           # find intents by keyword
cuz show cuz_8f3a1b            # full details of an intent
cuz status                     # coverage stats
cuz cost                       # token usage
cuz diff                       # intents for changed files
```

## Multi-session work

```sh
cuz parent start "migrate to gRPC"   # group intents under a goal
# ... work across multiple sessions ...
cuz parent end                        # close it out
```

## Uninstall

```sh
cuz teardown && brew uninstall cuz
```

Intent data in `.cuz/` stays in your repos — it's just committed JSON.
