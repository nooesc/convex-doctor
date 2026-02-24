use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Severity};

#[derive(Debug, Clone)]
pub struct ScoreResult {
    pub value: u32,
    pub label: &'static str,
}

pub fn compute_score(diagnostics: &[Diagnostic]) -> ScoreResult {
    let mut rule_deductions: HashMap<&str, (f64, f64)> = HashMap::new();

    for d in diagnostics {
        let (raw_per_instance, cap) = match d.severity {
            Severity::Error => (2.0, 4.0),
            Severity::Warning => (0.4, 1.5),
            Severity::Info => (0.0, 0.0),
        };
        let weight = d.category.weight();
        let entry = rule_deductions
            .entry(&d.rule)
            .or_insert((0.0, cap * weight));
        entry.0 += raw_per_instance * weight;
    }

    let total_deduction: f64 = rule_deductions
        .values()
        .map(|(raw, cap)| raw.min(*cap))
        .sum();

    let score_f64 = (100.0 - total_deduction).clamp(0.0, 100.0);
    let value = score_f64.round() as u32;

    let label = match value {
        85..=100 => "Healthy",
        70..=84 => "Needs attention",
        50..=69 => "Unhealthy",
        _ => "Critical",
    };

    ScoreResult { value, label }
}
