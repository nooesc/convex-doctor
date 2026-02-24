# Convex Doctor — Design Document

**Date:** 2026-02-23
**Status:** Approved

## Overview

Convex Doctor is a CLI tool that scans Convex backend codebases for security, performance, correctness, schema, architecture, and configuration issues. It outputs a 0-100 health score with actionable diagnostics. Inspired by [React Doctor](https://github.com/millionco/react-doctor).

## Goals

- Single Rust binary — fast, zero runtime dependencies
- 30+ rules across 6 categories covering common Convex anti-patterns
- Multiple output formats: interactive CLI, JSON, score-only, GitHub PR comments
- Config file support for rule toggling and severity overrides
- Diff mode for analyzing only changed files
- CI/CD integration via GitHub Actions
- AI agent integration (Claude Code, Cursor, etc.)

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  convex-doctor                   │
│                (single Rust binary)              │
├─────────────────────────────────────────────────┤
│  CLI Layer (clap)                                │
│  - Arg parsing, output formatting, scoring       │
│  - Modes: interactive, json, score-only, diff    │
├─────────────────────────────────────────────────┤
│  Orchestrator                                    │
│  - Project detection (framework, Convex version) │
│  - File discovery (glob convex/**/*.ts)          │
│  - Parallel file analysis via rayon              │
│  - Rule toggling based on project config         │
├─────────────────────────────────────────────────┤
│  Analysis Engine                                 │
│  - Oxc parser → AST per file                     │
│  - AST visitor pattern for rule evaluation       │
│  - Cross-file analysis (imports, schema refs)    │
├─────────────────────────────────────────────────┤
│  Rules (30+ rules across 6 categories)           │
│  - Each rule: pattern match on AST nodes         │
│  - Severity: error | warning                     │
│  - Help text: actionable fix description         │
├─────────────────────────────────────────────────┤
│  Reporters                                       │
│  - CLI (colored, categorized, ASCII score)       │
│  - JSON (structured diagnostics)                 │
│  - Score-only (single number for CI)             │
│  - GitHub PR comment (markdown)                  │
└─────────────────────────────────────────────────┘
```

### Key Rust Crates

- `oxc_parser` + `oxc_ast` — TypeScript parsing and AST types
- `clap` — CLI argument parsing
- `rayon` — parallel file analysis
- `ignore` — file discovery (respects `.gitignore`)
- `owo-colors` — terminal coloring
- `serde` + `serde_json` — JSON output
- `toml` — config file parsing

### Analysis Flow

1. Discover project: find `convex/` dir, read `convex.json`, detect framework
2. Load config: read `convex-doctor.toml` if present, merge with defaults
3. Glob all `.ts`/`.tsx`/`.js` files under `convex/` (skip `_generated/`)
4. Parse each file into AST (parallel via rayon)
5. Run all enabled rules against each file's AST
6. For cross-file rules (import analysis, schema coverage), build a file graph
7. Collect diagnostics, compute score, format output

## Rules

### Security (7 rules) — weight: 1.5x

| Rule ID | Description | Severity | Auto-fixable |
|---------|-------------|----------|-------------|
| `security/missing-arg-validators` | Public functions without `args: {}` | error | yes |
| `security/missing-return-validators` | Functions without `returns:` | warning | no |
| `security/missing-auth-check` | Public functions not calling `ctx.auth` | error | no |
| `security/internal-api-misuse` | Using `api.*` for scheduled/internal calls instead of `internal.*` | error | yes |
| `security/hardcoded-secrets` | Regex match for API keys, tokens, passwords in source | error | no |
| `security/env-not-gitignored` | `.env.local` not in `.gitignore` | error | yes |
| `security/spoofable-access-control` | Using user-provided args for access control instead of `ctx.auth` | warning | no |

### Performance (7 rules) — weight: 1.2x

| Rule ID | Description | Severity | Auto-fixable |
|---------|-------------|----------|-------------|
| `perf/unbounded-collect` | `.collect()` without `.take(n)` or pagination | error | no |
| `perf/filter-without-index` | `.filter()` on queries that could use `.withIndex()` | warning | no |
| `perf/date-now-in-query` | `Date.now()` in query functions | error | no |
| `perf/loop-run-mutation` | `ctx.runMutation` inside a loop | error | no |
| `perf/sequential-run-calls` | Multiple sequential `ctx.runQuery`/`ctx.runMutation` in actions | warning | no |
| `perf/unnecessary-run-action` | `ctx.runAction` when in same runtime | warning | no |
| `perf/helper-vs-run` | Using `ctx.runQuery`/`ctx.runMutation` inside queries/mutations instead of helpers | warning | no |

### Correctness (7 rules) — weight: 1.5x

| Rule ID | Description | Severity | Auto-fixable |
|---------|-------------|----------|-------------|
| `correctness/unwaited-promise` | Missing `await` on `ctx.scheduler`, `ctx.db.patch`, etc. | error | yes |
| `correctness/old-function-syntax` | `query(async (ctx) => ...)` instead of `query({ handler: ... })` | warning | yes |
| `correctness/db-in-action` | Using `ctx.db` directly in actions | error | no |
| `correctness/wrong-runtime-import` | Non-`"use node"` file importing `"use node"` file | error | no |
| `correctness/direct-function-ref` | Passing function directly instead of `api.*`/`internal.*` reference | error | no |
| `correctness/deprecated-api` | `v.bigint()`, `ctx.storage.getMetadata`, old cron syntax | warning | yes |
| `correctness/missing-unique` | Index query expecting single result without `.unique()` | warning | no |

### Schema (4 rules) — weight: 1.0x

| Rule ID | Description | Severity | Auto-fixable |
|---------|-------------|----------|-------------|
| `schema/missing-schema` | No `convex/schema.ts` file | warning | no |
| `schema/deep-nesting` | Deeply nested `v.object(v.array(v.object(...)))` (3+ levels) | warning | no |
| `schema/array-relationships` | Large arrays of `v.id("table")` for relationships | warning | no |
| `schema/redundant-index` | Index that's a prefix of another compound index | warning | no |

### Architecture (3 rules) — weight: 0.8x

| Rule ID | Description | Severity | Auto-fixable |
|---------|-------------|----------|-------------|
| `arch/large-handler` | Handler function body > 50 lines | warning | no |
| `arch/duplicated-auth` | Same auth pattern copy-pasted across 3+ functions | warning | no |
| `arch/monolithic-file` | Single file with 10+ exported functions | warning | no |

### Configuration (2 rules) — weight: 1.0x

| Rule ID | Description | Severity | Auto-fixable |
|---------|-------------|----------|-------------|
| `config/missing-convex-json` | No `convex.json` in project root | error | no |
| `config/missing-auth-config` | Auth used but no `auth.config.ts` | error | no |

## Scoring

```
score = 100 - (sum of deductions)

Deductions per diagnostic:
  error:   -3 points (capped contribution per rule at -15)
  warning: -1 point  (capped contribution per rule at -5)

Category weights (multiplier on deductions):
  security:      1.5x
  performance:   1.2x
  correctness:   1.5x
  schema:        1.0x
  architecture:  0.8x
  configuration: 1.0x

Floor: 0  |  Ceiling: 100

Labels:
  85-100: "Healthy"
  70-84:  "Needs attention"
  50-69:  "Unhealthy"
  0-49:   "Critical"
```

## Output Formats

### Interactive CLI (default)

Colored output with branded header, score, categorized diagnostics with file locations and actionable help text. Verbose mode (`--verbose`) shows all affected locations.

### JSON (`--format json`)

Structured output with project info, score, summary counts, and full diagnostic array. Suitable for AI agent consumption and programmatic use.

### Score-only (`--score`)

Single integer on stdout. Useful for CI gating (`convex-doctor --score | test $(cat) -ge 70`).

### Diff mode (`--diff [base]`)

Only analyzes files changed vs a base branch. Uses `git diff --name-only <base>` to determine changed files.

### GitHub Actions

GitHub Action that runs convex-doctor, posts a PR comment with score and top issues, and fails the check if score is below a configurable threshold.

## Configuration

`convex-doctor.toml` in project root:

```toml
[rules]
"perf/unbounded-collect" = "off"
"arch/large-handler" = { severity = "error", max_lines = 100 }

[ignore]
files = ["convex/_generated/**", "convex/test/**"]

[ci]
fail_below = 70
```

## Distribution

- **GitHub Releases**: Pre-built binaries for x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin, x86_64-windows
- **Install script**: `curl -fsSL https://convex.doctor/install.sh | sh`
- **cargo install**: `cargo install convex-doctor`
- **Homebrew**: Stretch goal
- Cross-compilation via GitHub Actions matrix builds

## References

- [React Doctor](https://github.com/millionco/react-doctor) — inspiration
- [Oxc](https://oxc.rs/) — TypeScript parser
- [Convex Best Practices](https://docs.convex.dev/understanding/best-practices/)
- [Convex ESLint Plugin](https://www.npmjs.com/package/@convex-dev/eslint-plugin)
- [Convex Authorization](https://stack.convex.dev/authorization)
