[![CI](https://github.com/exla-ai/cuz/actions/workflows/ci.yml/badge.svg)](https://github.com/exla-ai/cuz/actions/workflows/ci.yml)

# cuz

Give every piece of code a traceable reason for existing.

cuz injects into Claude Code so that every commit automatically captures *why* the change was made — not just what changed. A companion CLI makes that reasoning queryable, traceable, and permanent.

## Install

```sh
brew install exla-ai/cuz/cuz
```

That's it. No setup step. Brew post-install automatically:
- Patches `~/.claude/CLAUDE.md` with intent tracking instructions
- Installs a PostToolUse hook in `~/.claude/settings.json`

Then `cuz init` in any repo to start tracking.

## Commands

```sh
cuz init                          # Initialize .cuz/ in current repo
cuz why src/retry.ts:42           # Why does this line exist?
cuz show cuz_8f3a1b               # Show full details of an intent
cuz search "retry"                # Search intents by keyword
cuz log                           # Show intent history (tree view)
cuz log --json                    # Machine-readable output
cuz status                        # Coverage stats + progress bar
cuz cost                          # Token usage across intents
cuz diff                          # Show intents for changed files
cuz diff --cached                 # Show intents for staged files
cuz parent start "migrate to gRPC"  # Start multi-session goal
cuz parent show                   # Show active parent
cuz parent end                    # End active parent
cuz setup                         # Re-run Claude Code integration
cuz teardown                      # Remove from Claude Code (keeps data)
```

## How it works

1. **On every commit**, Claude Code creates a `.cuz/intents/cuz_XXXXXX.json` recording the goal, approach, alternatives considered, and confidence level
2. **An `Intent: cuz_XXXXXX` trailer** in the commit message links the commit to its reasoning
3. **`cuz why`** follows git blame to find the intent behind any line of code — walking history if the direct commit lacks a trailer
4. **Before modifying code**, Claude reads existing intents to understand why code exists and what was already rejected
5. **`cuz verify`** runs as a PostToolUse hook — warns Claude (never blocks) when a commit is missing its intent trailer

## Data model

```
.cuz/
  intents/
    cuz_8f3a1b.json          # intent records (reasoning)
  parents/
    cuz_parent_f1a2b3.json   # multi-session parent intents
  active_parent               # current parent intent ID
  schema.json                 # schema version
```

Intent records contain:
- **goal** — what the user asked for, in their words
- **approach** — what was done and why
- **alternatives** — options considered and why they were rejected
- **confidence** — 0–1 score
- **token_cost** — approximate tokens used
- **parent_intent** — links to multi-session goals

## Uninstall

```sh
cuz teardown        # removes Claude Code integration
brew uninstall cuz  # removes the binary
# .cuz/ directories in repos are left intact (it's just data)
```
