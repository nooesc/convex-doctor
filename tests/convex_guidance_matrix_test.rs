use convex_doctor::rules::RuleRegistry;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
struct CoverageMatrix {
    version: String,
    guidelines: Vec<GuidelineEntry>,
}

#[derive(Debug, Deserialize)]
struct GuidelineEntry {
    id: String,
    source: String,
    source_line: u32,
    statement: String,
    enforceable: bool,
    rule_id: Option<String>,
    severity_tiered: Option<String>,
    severity_strict: Option<String>,
    severity_low_noise: Option<String>,
    notes: Option<String>,
}

fn normalize_statement(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_guideline_bullets(path: &str) -> Vec<(u32, String)> {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path));

    let mut bullets = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let line_no = (index + 1) as u32;
        let trimmed = line.trim();
        if trimmed.starts_with("# Examples:") {
            break;
        }
        if let Some(statement) = trimmed.strip_prefix("- ") {
            bullets.push((line_no, normalize_statement(statement)));
        }
    }
    bullets
}

#[test]
fn enforceable_guidelines_reference_existing_rules() {
    let raw = std::fs::read_to_string("reference/convex/coverage_matrix.toml").unwrap();
    let matrix: CoverageMatrix = toml::from_str(&raw).unwrap();
    assert_eq!(matrix.version, "v0.241.0");

    let registry = RuleRegistry::new();
    let known_rule_ids: HashSet<&str> = registry.rules().iter().map(|rule| rule.id()).collect();
    let valid_severities = ["off", "info", "warning", "error"];

    let mut seen_ids = HashSet::new();
    let mut seen_source_lines = HashSet::new();

    for guideline in &matrix.guidelines {
        assert!(
            seen_ids.insert(guideline.id.as_str()),
            "duplicate guideline id `{}`",
            guideline.id
        );
        assert!(
            matches!(
                guideline.source.as_str(),
                "convex_rules" | "convex_instructions"
            ),
            "guideline `{}` has unexpected source `{}`",
            guideline.id,
            guideline.source
        );
        assert!(
            seen_source_lines.insert((guideline.source.as_str(), guideline.source_line)),
            "duplicate source/source_line pair `{}`:{}",
            guideline.source,
            guideline.source_line
        );
        assert!(
            !guideline.statement.trim().is_empty(),
            "guideline `{}` has empty statement",
            guideline.id
        );
        assert_eq!(
            guideline.statement,
            normalize_statement(&guideline.statement),
            "guideline `{}` has non-normalized statement whitespace",
            guideline.id
        );
        assert!(
            guideline
                .notes
                .as_deref()
                .is_some_and(|notes| !notes.trim().is_empty()),
            "guideline `{}` must include notes",
            guideline.id
        );

        if guideline.enforceable {
            let rule_id = guideline.rule_id.as_deref().unwrap_or_else(|| {
                panic!("enforceable guideline `{}` missing rule_id", guideline.id)
            });
            assert!(
                known_rule_ids.contains(rule_id),
                "coverage matrix rule `{}` for guideline `{}` not found in registry",
                rule_id,
                guideline.id
            );

            for (label, severity) in [
                ("severity_tiered", guideline.severity_tiered.as_deref()),
                ("severity_strict", guideline.severity_strict.as_deref()),
                (
                    "severity_low_noise",
                    guideline.severity_low_noise.as_deref(),
                ),
            ] {
                let value = severity.unwrap_or_else(|| {
                    panic!("enforceable guideline `{}` missing {}", guideline.id, label)
                });
                assert!(
                    valid_severities.contains(&value),
                    "guideline `{}` has invalid {} `{}`",
                    guideline.id,
                    label,
                    value
                );
            }
        } else {
            assert!(
                guideline.rule_id.is_none(),
                "non-enforceable guideline `{}` should not set rule_id",
                guideline.id
            );
            assert!(
                guideline.severity_tiered.is_none()
                    && guideline.severity_strict.is_none()
                    && guideline.severity_low_noise.is_none(),
                "non-enforceable guideline `{}` should not set severity overrides",
                guideline.id
            );
        }
    }
}

#[test]
fn matrix_exhaustively_covers_normative_bullets() {
    let raw = std::fs::read_to_string("reference/convex/coverage_matrix.toml").unwrap();
    let matrix: CoverageMatrix = toml::from_str(&raw).unwrap();

    let convex_rules_bullets =
        extract_guideline_bullets("reference/convex/convex_rules.v0.241.0.txt");
    let convex_instructions_bullets =
        extract_guideline_bullets("reference/convex/convex.instructions.v0.241.0.md");

    let rules_texts: HashSet<&str> = convex_rules_bullets
        .iter()
        .map(|(_, statement)| statement.as_str())
        .collect();
    let instructions_texts: HashSet<&str> = convex_instructions_bullets
        .iter()
        .map(|(_, statement)| statement.as_str())
        .collect();
    assert_eq!(
        rules_texts, instructions_texts,
        "pinned convex_rules and convex.instructions snapshots diverged; update matrix coverage"
    );

    let matrix_rules_only: Vec<&GuidelineEntry> = matrix
        .guidelines
        .iter()
        .filter(|guideline| guideline.source == "convex_rules")
        .collect();
    assert_eq!(
        matrix_rules_only.len(),
        convex_rules_bullets.len(),
        "matrix must include one convex_rules row per pre-Examples bullet"
    );

    let mut expected_by_line = HashMap::new();
    for (line, statement) in convex_rules_bullets {
        let previous = expected_by_line.insert(line, statement);
        assert!(
            previous.is_none(),
            "duplicate extracted bullet at convex_rules line {}",
            line
        );
    }

    let mut matrix_by_line = HashMap::new();
    for guideline in matrix_rules_only {
        let previous = matrix_by_line.insert(guideline.source_line, guideline.statement.as_str());
        assert!(
            previous.is_none(),
            "duplicate matrix row for convex_rules line {}",
            guideline.source_line
        );
    }

    for (line, expected_statement) in expected_by_line {
        let actual_statement = matrix_by_line
            .get(&line)
            .unwrap_or_else(|| panic!("missing matrix row for convex_rules line {}", line));
        assert_eq!(
            *actual_statement, expected_statement,
            "matrix statement mismatch at convex_rules line {}",
            line
        );
    }
}
