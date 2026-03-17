[![CI](https://github.com/exla-ai/cuz/actions/workflows/ci.yml/badge.svg)](https://github.com/exla-ai/cuz/actions/workflows/ci.yml)
[![phase: 1](https://img.shields.io/badge/phase-1%20%E2%80%93%20core%20CLI-22c55e)](https://github.com/exla-ai/cuz)
[![intent coverage](https://img.shields.io/badge/intent%20coverage-ready-22c55e)](https://github.com/exla-ai/cuz)

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
- Initializes `.cuz/` in your current repo (if inside one)

## Usage

```sh
# Why does this line exist?
cuz why src/payments/retry.ts:42

# Show intent history
cuz log

# Check tracking coverage
cuz status
```

## How it works

1. **On every commit**, Claude Code creates a `.cuz/intents/cuz_XXXXXX.json` recording the goal, approach, alternatives considered, and confidence level
2. **An `Intent: cuz_XXXXXX` trailer** in the commit message links the commit to its reasoning
3. **`cuz why`** follows git blame to find the intent behind any line of code — walking history if the direct commit lacks a trailer
4. **Before modifying code**, Claude reads existing intents to understand why code exists and what was already rejected

## Data model

```
.cuz/
  intents/
    cuz_8f3a1b.json     # intent records (reasoning)
  parents/
    cuz_parent_f1a2b3.json  # multi-session parent intents
  active_parent          # current parent intent ID
  schema.json            # schema version
```

Intent records contain:
- **goal** — what the user asked for, in their words
- **approach** — what was done and why
- **alternatives** — options considered and why they were rejected
- **confidence** — 0–1 score
- **token_cost** — approximate tokens used
- **parent_intent** — links to multi-session goals

## Phase 1 status

- [x] Core CLI (`setup`, `verify`, `why`, `log`, `status`)
- [x] CLAUDE.md injection (idempotent, marker-based)
- [x] PostToolUse hook (fast bail, never blocks Claude)
- [x] Git trailer parsing + blame history walking
- [x] 29 tests passing (16 unit + 13 integration)
- [x] CI + release workflows
- [x] Homebrew formula
