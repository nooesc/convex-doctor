#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/reference/convex"
DEFAULT_VERSION="v0.241.0"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<EOF
Usage: scripts/update-convex-guidance.sh [vMAJOR.MINOR.PATCH]

Downloads latest Convex guidance snapshots and regenerates:
  - reference/convex/convex_rules.<version>.txt
  - reference/convex/convex.instructions.<version>.md
  - reference/convex/coverage_matrix.toml

If version is omitted, defaults to ${DEFAULT_VERSION}.
EOF
  exit 0
fi

normalize_version() {
  local raw="${1:-$DEFAULT_VERSION}"
  if [[ "$raw" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf "%s" "$raw"
    return
  fi
  if [[ "$raw" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf "v%s" "$raw"
    return
  fi
  echo "Invalid version '$raw'. Expected vMAJOR.MINOR.PATCH (example: v0.241.0)." >&2
  exit 1
}

VERSION="$(normalize_version "${1:-$DEFAULT_VERSION}")"
RULES_FILE="$OUT_DIR/convex_rules.${VERSION}.txt"
INSTRUCTIONS_FILE="$OUT_DIR/convex.instructions.${VERSION}.md"
MATRIX_FILE="$OUT_DIR/coverage_matrix.toml"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

TMP_RULES="$TMP_DIR/convex_rules.txt"
TMP_INSTRUCTIONS="$TMP_DIR/convex.instructions.md"
TMP_MATRIX="$TMP_DIR/coverage_matrix.toml"

extract_bullets() {
  local file="$1"
  awk '
    /^# Examples:/ { exit }
    /^- / {
      statement = $0
      sub(/^- /, "", statement)
      gsub(/[[:space:]]+/, " ", statement)
      sub(/^ /, "", statement)
      sub(/ $/, "", statement)
      print statement
    }
  ' "$file"
}

mkdir -p "$OUT_DIR"

echo "Updating pinned Convex guidance snapshots for $VERSION..."
curl -fsSL https://convex.link/convex_rules.txt >"$TMP_RULES"
curl -fsSL https://convex.link/convex_github_copilot_instructions >"$TMP_INSTRUCTIONS"

extract_bullets "$TMP_RULES" | sort -u >"$TMP_DIR/rules_bullets.txt"
extract_bullets "$TMP_INSTRUCTIONS" | sort -u >"$TMP_DIR/instructions_bullets.txt"

if ! diff -u "$TMP_DIR/rules_bullets.txt" "$TMP_DIR/instructions_bullets.txt" >"$TMP_DIR/bullet_diff.txt"; then
  echo "Pinned Convex guidance snapshots diverged before # Examples; not regenerating coverage matrix." >&2
  cat "$TMP_DIR/bullet_diff.txt" >&2
  exit 1
fi

if [[ ! -f "$MATRIX_FILE" ]]; then
  cat >"$MATRIX_FILE" <<'EOF'
version = "v0.0.0"
guidelines = []
EOF
fi

awk -v version="$VERSION" '
function trim(s) {
  sub(/^[[:space:]]+/, "", s)
  sub(/[[:space:]]+$/, "", s)
  return s
}

function normalize_ws(s) {
  gsub(/[[:space:]]+/, " ", s)
  return trim(s)
}

function escape_toml(s) {
  gsub(/\\/, "\\\\", s)
  gsub(/"/, "\\\"", s)
  return s
}

function unescape_toml(s, sentinel) {
  sentinel = "\034"
  gsub(/\\\\/, sentinel, s)
  gsub(/\\"/, "\"", s)
  gsub(sentinel, "\\", s)
  return s
}

function extract_quoted_value(line, value) {
  value = line
  sub(/^[^"]*"/, "", value)
  sub(/"$/, "", value)
  return unescape_toml(value)
}

function flush_existing_entry(key) {
  if (!in_existing_entry || existing_source != "convex_rules" || existing_statement == "") {
    return
  }
  key = normalize_ws(existing_statement)
  previous_enforceable[key] = existing_enforceable
  previous_rule_id[key] = existing_rule_id
  previous_severity_tiered[key] = existing_severity_tiered
  previous_severity_strict[key] = existing_severity_strict
  previous_severity_low_noise[key] = existing_severity_low_noise
  previous_notes[key] = existing_notes
}

function print_non_enforceable(notes) {
  print "enforceable = false"
  printf "notes = \"%s\"\n\n", escape_toml(notes)
}

function print_enforceable(rule_id, severity_tiered, severity_strict, severity_low_noise, notes) {
  print "enforceable = true"
  printf "rule_id = \"%s\"\n", escape_toml(rule_id)
  printf "severity_tiered = \"%s\"\n", escape_toml(severity_tiered)
  printf "severity_strict = \"%s\"\n", escape_toml(severity_strict)
  printf "severity_low_noise = \"%s\"\n", escape_toml(severity_low_noise)
  printf "notes = \"%s\"\n\n", escape_toml(notes)
}

FNR == NR {
  if ($0 ~ /^\[\[guidelines\]\]/) {
    flush_existing_entry()
    in_existing_entry = 1
    existing_source = ""
    existing_statement = ""
    existing_enforceable = ""
    existing_rule_id = ""
    existing_severity_tiered = ""
    existing_severity_strict = ""
    existing_severity_low_noise = ""
    existing_notes = ""
    next
  }

  if (!in_existing_entry) {
    next
  }

  if ($0 ~ /^source = /) {
    existing_source = extract_quoted_value($0)
    next
  }
  if ($0 ~ /^statement = /) {
    existing_statement = extract_quoted_value($0)
    next
  }
  if ($0 ~ /^enforceable = /) {
    existing_enforceable = trim(substr($0, index($0, "=") + 1))
    next
  }
  if ($0 ~ /^rule_id = /) {
    existing_rule_id = extract_quoted_value($0)
    next
  }
  if ($0 ~ /^severity_tiered = /) {
    existing_severity_tiered = extract_quoted_value($0)
    next
  }
  if ($0 ~ /^severity_strict = /) {
    existing_severity_strict = extract_quoted_value($0)
    next
  }
  if ($0 ~ /^severity_low_noise = /) {
    existing_severity_low_noise = extract_quoted_value($0)
    next
  }
  if ($0 ~ /^notes = /) {
    existing_notes = extract_quoted_value($0)
    next
  }
  next
}

{
  if (FNR == 1) {
    flush_existing_entry()
    print "version = \"" version "\"\n"
    print "# Exhaustive coverage of every bullet guideline before `# Examples:`"
    print "# in reference/convex/convex_rules." version ".txt."
    print "# Generated by scripts/update-convex-guidance.sh.\n"
  }

  if ($0 ~ /^# Examples:/) {
    exit
  }

  if ($0 !~ /^- /) {
    next
  }

  line_number = FNR
  statement = $0
  sub(/^- /, "", statement)
  statement = normalize_ws(statement)
  key = statement

  print "[[guidelines]]"
  printf "id = \"convex_rules.l%03d\"\n", line_number
  print "source = \"convex_rules\""
  printf "source_line = %d\n", line_number
  printf "statement = \"%s\"\n", escape_toml(statement)

  if (!(key in previous_enforceable)) {
    print_non_enforceable("TODO: New guideline bullet not yet classified for enforceability.")
    new_statement_count++
    next
  }

  if (previous_enforceable[key] != "true") {
    notes = previous_notes[key]
    if (notes == "") {
      notes = "Not currently enforceable with this static analysis engine without higher-fidelity semantic/runtime context."
    }
    print_non_enforceable(notes)
    next
  }

  if (previous_rule_id[key] == "" ||
      previous_severity_tiered[key] == "" ||
      previous_severity_strict[key] == "" ||
      previous_severity_low_noise[key] == "") {
    print_non_enforceable("TODO: Previous enforceable mapping was incomplete; review and set rule_id/severities.")
    incomplete_mapping_count++
    next
  }

  notes = previous_notes[key]
  if (notes == "") {
    notes = "Mapped to existing rule from prior coverage matrix."
  }
  print_enforceable(previous_rule_id[key], previous_severity_tiered[key], previous_severity_strict[key], previous_severity_low_noise[key], notes)
}

END {
  if (new_statement_count > 0) {
    printf "NOTE: %d new guideline bullet(s) were marked TODO.\n", new_statement_count > "/dev/stderr"
  }
  if (incomplete_mapping_count > 0) {
    printf "NOTE: %d prior mapping(s) were incomplete and marked TODO.\n", incomplete_mapping_count > "/dev/stderr"
  }
}
' "$MATRIX_FILE" "$TMP_RULES" >"$TMP_MATRIX"

mv "$TMP_RULES" "$RULES_FILE"
mv "$TMP_INSTRUCTIONS" "$INSTRUCTIONS_FILE"
mv "$TMP_MATRIX" "$MATRIX_FILE"

echo "Updated:"
echo "  - $RULES_FILE"
echo "  - $INSTRUCTIONS_FILE"
echo "  - $MATRIX_FILE"
