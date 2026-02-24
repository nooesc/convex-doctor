# convex-doctor

Diagnose your Convex backend for anti-patterns, security issues, and performance problems.

`convex-doctor` is a static analysis CLI for [Convex](https://convex.dev) projects. It parses your `convex/` directory, runs 30 rules across 6 categories, and produces a weighted 0-100 health score. Think of it as ESLint, but purpose-built for Convex backends.

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
# Note: diff mode only runs file-level rules on changed files.

# Scan a specific project path
convex-doctor /path/to/my-project
```

## Rules

convex-doctor runs 30 rules organized into 6 categories. Each category carries a different weight in the final score.

| Category | Weight | Rules | Description |
|---|---|---|---|
| **Security** | 1.5x | 7 | Argument/return validators, auth checks, internal API misuse, hardcoded secrets, env files, access control |
| **Performance** | 1.2x | 7 | Unbounded collect, filter without index, Date.now() in queries, loop mutations, sequential runs, action-in-action, helper vs run |
| **Correctness** | 1.5x | 7 | Unwaited promises, old function syntax, db access in actions, deprecated APIs, runtime imports, function refs, missing unique |
| **Schema** | 1.0x | 4 | Missing schema, deep nesting, array relationships, redundant indexes |
| **Architecture** | 0.8x | 3 | Large handler functions, monolithic files, duplicated auth patterns |
| **Configuration** | 1.0x | 2 | Missing convex.json, missing auth config |

### Rule reference

| Rule ID | Severity | What it detects |
|---|---|---|
| `security/missing-arg-validators` | error | Public functions without `args` validators |
| `security/missing-return-validators` | warning | Functions without `returns` validators |
| `security/missing-auth-check` | warning | Public functions that never call `ctx.auth.getUserIdentity()` |
| `security/internal-api-misuse` | error | Server-to-server calls using `api.*` instead of `internal.*` |
| `security/hardcoded-secrets` | error | API keys, tokens, or secrets hardcoded in source |
| `security/env-not-gitignored` | error | `.env.local` exists but is not in `.gitignore` |
| `security/spoofable-access-control` | warning | Access control based on spoofable client arguments (stub) |
| `perf/unbounded-collect` | error | `.collect()` without `.take(n)` limit |
| `perf/filter-without-index` | warning | `.filter()` calls that scan entire tables |
| `perf/date-now-in-query` | error | `Date.now()` in query functions (breaks caching) |
| `perf/loop-run-mutation` | error | `ctx.runMutation`/`ctx.runQuery` inside loops (N+1) |
| `perf/sequential-run-calls` | warning | Multiple sequential `ctx.run*` calls in an action |
| `perf/unnecessary-run-action` | warning | `ctx.runAction` called from within an action |
| `perf/helper-vs-run` | warning | `ctx.runQuery`/`ctx.runMutation` inside a query or mutation |
| `correctness/unwaited-promise` | error | `ctx.db.insert`, `ctx.runMutation`, etc. without `await` |
| `correctness/old-function-syntax` | warning | Legacy function registration syntax |
| `correctness/db-in-action` | error | Direct `ctx.db.*` calls inside actions |
| `correctness/deprecated-api` | warning | Usage of deprecated Convex APIs |
| `correctness/wrong-runtime-import` | warning | Imports across incompatible Convex runtimes (stub) |
| `correctness/direct-function-ref` | warning | Direct function references instead of `api.*` references (stub) |
| `correctness/missing-unique` | warning | `.first()` on indexed query where `.unique()` may be more appropriate |
| `schema/missing-schema` | warning | No `schema.ts` file found in `convex/` directory |
| `schema/deep-nesting` | warning | Schema validators nested more than 3 levels deep |
| `schema/array-relationships` | warning | `v.array(v.id(...))` patterns that may grow unbounded |
| `schema/redundant-index` | warning | Index that is a prefix of another index on the same table |
| `arch/large-handler` | warning | Handler functions exceeding 50 lines |
| `arch/monolithic-file` | warning | Files with more than 10 exported functions |
| `arch/duplicated-auth` | warning | 3+ functions with inline auth checks in the same file |
| `config/missing-convex-json` | warning | No `convex.json` found in project root |
| `config/missing-auth-config` | error | Functions use `ctx.auth` but no `auth.config.ts` exists |

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
