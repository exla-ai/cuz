# cuz

## Releasing

When you believe significant changes have accumulated — new commands, important bug fixes, or meaningful behavior changes — propose a new release to the user. Don't release for docs-only or intent-only changes.

To cut a release:

1. Bump `version` in `Cargo.toml`
2. Update `VERSION` in `install.sh` to match (e.g. `v0.2.0`)
3. Commit with message: `Release vX.Y.Z`
4. Tag: `git tag vX.Y.Z`
5. Push: `git push origin main --tags`

The release workflow (.github/workflows/release.yml) will build binaries for macOS (ARM + Intel) and Linux, then create a GitHub release with the assets.

## Project structure

- `src/` — Rust CLI source
- `src/commands/` — one file per subcommand
- `src/prompt.rs` — the CLAUDE.md prompt that gets injected by `cuz setup`
- `install.sh` — curl installer (downloads binary + runs `cuz setup`)
- `.cuz/intents/` — intent records for this repo
