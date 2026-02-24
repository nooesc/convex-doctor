# convex-doctor

Diagnose your Convex backend for anti-patterns, security issues, and performance problems.

`convex-doctor` is a static analysis CLI for [Convex](https://convex.dev) projects. It parses your `convex/` directory, runs 15 rules across 4 categories, and produces a weighted 0-100 health score. Think of it as ESLint, but purpose-built for Convex backends.

## Quick start

### Install from crates.io

```sh
cargo install convex-doctor
```

### Build from source

```sh
git clone https://github.com/coler/convex-doctor.git
cd convex-doctor
cargo build --release
# Binary is at ./target/release/convex-doctor
```

## Usage

Run from your project root (the directory containing `convex/`):

```sh
# Basic scan
convex-doctor

# Verbose output with file paths and line numbers
convex-doctor -v

# JSON output (for CI or tooling)
convex-doctor --format json

# Score only (prints just the number, e.g. "87")
convex-doctor --score

# Diff mode: only analyze files changed vs a base branch
convex-doctor --diff main

# Scan a specific project path
convex-doctor /path/to/my-project
```

## Rules

convex-doctor runs 15 rules organized into 4 categories. Each category carries a different weight in the final score.

| Category | Weight | Rules | Description |
|---|---|---|---|
| **Security** | 1.5x | 5 | Argument/return validators, auth checks, internal API misuse, hardcoded secrets |
| **Performance** | 1.2x | 4 | Unbounded collect, filter without index, Date.now() in queries, loop mutations |
| **Correctness** | 1.5x | 4 | Unwaited promises, old function syntax, db access in actions, deprecated APIs |
| **Architecture** | 0.8x | 2 | Large handler functions, monolithic files |

### Rule reference

| Rule ID | Severity | What it detects |
|---|---|---|
| `security/missing-arg-validators` | error | Public functions without `args` validators |
| `security/missing-return-validators` | warning | Functions without `returns` validators |
| `security/missing-auth-check` | warning | Public functions that never call `ctx.auth.getUserIdentity()` |
| `security/internal-api-misuse` | warning | Server-to-server calls using `api.*` instead of `internal.*` |
| `security/hardcoded-secrets` | error | API keys, tokens, or secrets hardcoded in source |
| `perf/unbounded-collect` | error | `.collect()` without `.take(n)` limit |
| `perf/filter-without-index` | warning | `.filter()` calls that scan entire tables |
| `perf/date-now-in-query` | error | `Date.now()` in query functions (breaks caching) |
| `perf/loop-run-mutation` | error | `ctx.runMutation`/`ctx.runQuery` inside loops (N+1) |
| `correctness/unwaited-promise` | error | `ctx.db.insert`, `ctx.runMutation`, etc. without `await` |
| `correctness/old-function-syntax` | warning | Legacy function registration syntax |
| `correctness/db-in-action` | error | Direct `ctx.db.*` calls inside actions |
| `correctness/deprecated-api` | warning | Usage of deprecated Convex APIs |
| `arch/large-handler` | warning | Handler functions exceeding 50 lines |
| `arch/monolithic-file` | warning | Files with more than 10 exported functions |

## Scoring

The health score ranges from 0 to 100. Each finding deducts points based on its severity and category weight, with per-rule caps to prevent a single noisy rule from dominating the score.

| Score | Label | Meaning |
|---|---|---|
| 85 - 100 | Healthy | Few or no issues detected |
| 70 - 84 | Needs attention | Some issues worth addressing |
| 50 - 69 | Unhealthy | Significant problems found |
| 0 - 49 | Critical | Serious issues requiring immediate attention |

## Example output

```
  convex-doctor v0.1.0

  Project: my-app

  Score: 72 / 100 — Needs attention

  4 errors, 3 warnings

  ── Security ──────────────────────────────────────────
  ERROR  security/missing-arg-validators
         Public query `getUser` has no argument validators
         Help: Add `args: { ... }` with validators for all parameters.

   WARN  security/missing-auth-check
         Public mutation `createPost` does not check authentication
         Help: Consider adding `const identity = await ctx.auth.getUserIdentity()`.

  ── Performance ───────────────────────────────────────
  ERROR  perf/unbounded-collect
         Unbounded `.collect()` call
         Help: Use `.take(n)` to limit results or implement pagination.

  ── Correctness ───────────────────────────────────────
  ERROR  correctness/unwaited-promise
         `ctx.db.insert` is not awaited
         Help: This call returns a Promise that must be awaited.
```

## Configuration

Create a `convex-doctor.toml` in your project root to customize behavior:

```toml
# Disable specific rules
[rules]
"security/missing-return-validators" = "off"
"arch/monolithic-file" = "off"

# Ignore files by glob pattern
[ignore]
files = [
  "convex/_generated/**",
  "convex/testHelpers.ts",
]

# CI: exit with code 1 if score is below threshold
[ci]
fail_below = 70
```

## CI integration

### GitHub Actions

Add convex-doctor to your CI pipeline to catch regressions:

```yaml
name: Convex Health Check
on: [pull_request]

jobs:
  convex-doctor:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install convex-doctor
        run: cargo install convex-doctor

      - name: Run convex-doctor
        run: |
          SCORE=$(convex-doctor --score)
          echo "Health score: $SCORE"
          if [ "$SCORE" -lt 70 ]; then
            echo "::error::Convex health score is $SCORE (below threshold of 70)"
            convex-doctor -v
            exit 1
          fi
```

You can also use the `[ci]` config section in `convex-doctor.toml` to set the threshold, then simply run:

```yaml
      - name: Run convex-doctor
        run: convex-doctor
```

If `fail_below` is set in the config, convex-doctor will exit with code 1 when the score falls below the threshold.

## Contributing

Contributions are welcome. To get started:

```sh
git clone https://github.com/coler/convex-doctor.git
cd convex-doctor
cargo test
cargo clippy -- -D warnings
```

1. Fork the repository
2. Create a feature branch (`git checkout -b my-feature`)
3. Run `cargo test` and `cargo clippy -- -D warnings` before committing
4. Open a pull request

## License

MIT
