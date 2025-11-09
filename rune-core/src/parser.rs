//! Parser for RUNE configuration files

use crate::datalog::types::{Atom as DatalogAtom, Rule as DatalogRule, Term as DatalogTerm};
use crate::error::{RUNEError, Result};
use crate::types::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Parsed RUNE configuration
#[derive(Debug, Clone)]
pub struct RUNEConfig {
    /// Version string
    pub version: String,
    /// Data section (TOML-style)
    pub data: toml::Value,
    /// Datalog rules (not serializable as they're parsed at runtime)
    pub rules: Vec<DatalogRule>,
    /// Cedar policies
    pub policies: Vec<Policy>,
}

/// A Cedar policy in the RUNE file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy ID
    pub id: String,
    /// Policy content
    pub content: String,
}

/// Parse a RUNE configuration file
pub fn parse_rune_file(input: &str) -> Result<RUNEConfig> {
    // Split file into sections
    let sections = split_sections(input)?;

    // Parse version
    let version = sections
        .version
        .ok_or_else(|| RUNEError::ParseError("Missing version declaration".into()))?;

    // Parse data section as TOML
    let data = if let Some(data_str) = sections.data {
        toml::from_str(&data_str)
            .map_err(|e| RUNEError::ParseError(format!("Failed to parse data section: {}", e)))?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // Parse rules (simplified for now)
    let rules = if let Some(rules_str) = sections.rules {
        parse_rules(&rules_str)?
    } else {
        Vec::new()
    };

    // Parse policies
    let policies = if let Some(policies_str) = sections.policies {
        parse_policies(&policies_str)?
    } else {
        Vec::new()
    };

    Ok(RUNEConfig {
        version,
        data,
        rules,
        policies,
    })
}

/// Sections in a RUNE file
struct Sections {
    version: Option<String>,
    data: Option<String>,
    rules: Option<String>,
    policies: Option<String>,
}

/// Split input into sections
fn split_sections(input: &str) -> Result<Sections> {
    let mut sections = Sections {
        version: None,
        data: None,
        rules: None,
        policies: None,
    };

    let mut current_section = None;
    let mut section_content = String::new();

    for line in input.lines() {
        if line.starts_with("version") {
            // Save previous section
            save_section(&mut sections, current_section, &section_content);
            section_content.clear();

            // Extract version
            if let Some(version) = line.split('=').nth(1) {
                sections.version = Some(version.trim().trim_matches('"').to_string());
            }
            current_section = None;
        } else if line.starts_with("[data]") {
            save_section(&mut sections, current_section, &section_content);
            section_content.clear();
            current_section = Some("data");
        } else if line.starts_with("[rules]") {
            save_section(&mut sections, current_section, &section_content);
            section_content.clear();
            current_section = Some("rules");
        } else if line.starts_with("[policies]") {
            save_section(&mut sections, current_section, &section_content);
            section_content.clear();
            current_section = Some("policies");
        } else if current_section.is_some() {
            section_content.push_str(line);
            section_content.push('\n');
        }
    }

    // Save last section
    save_section(&mut sections, current_section, &section_content);

    Ok(sections)
}

/// Save section content
fn save_section(sections: &mut Sections, section_name: Option<&str>, content: &str) {
    if content.is_empty() {
        return;
    }

    match section_name {
        Some("data") => sections.data = Some(content.to_string()),
        Some("rules") => sections.rules = Some(content.to_string()),
        Some("policies") => sections.policies = Some(content.to_string()),
        _ => {}
    }
}

/// Split a string by commas, but only at the top level (not inside parentheses)
fn split_preserving_parens(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut current_start = 0;
    let mut depth = 0;

    for (i, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&input[current_start..i]);
                current_start = i + 1;
            }
            _ => {}
        }
    }

    // Add the last part
    if current_start < input.len() {
        parts.push(&input[current_start..]);
    }

    parts
}

/// Parse Datalog rules
pub fn parse_rules(input: &str) -> Result<Vec<DatalogRule>> {
    let mut rules = Vec::new();
    let mut current_rule = String::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Accumulate lines for the current rule
        if !current_rule.is_empty() {
            current_rule.push(' ');
        }
        current_rule.push_str(line);

        // Check if rule is complete (ends with period)
        if current_rule.trim_end().ends_with('.') {
            // Parse the complete rule
            let rule_str = current_rule.trim();

            // Check if this is a fact (no body) or a rule (has :-)
            if let Some((head, body)) = rule_str.split_once(":-") {
                // Rule with head and body
                let head_atom = parse_atom(head.trim(), false)?;
                let body_str = body.trim().trim_end_matches('.');
                let body_atoms = split_preserving_parens(body_str)
                    .into_iter()
                    .map(|s| {
                        let s = s.trim();
                        // Check for negation
                        let negated = s.starts_with("not ");
                        let atom_str = if negated { &s[4..] } else { s };
                        parse_atom(atom_str.trim(), negated)
                    })
                    .collect::<Result<Vec<_>>>()?;

                rules.push(DatalogRule::new(head_atom, body_atoms));
            } else {
                // Fact (ground atom with no body)
                let fact_atom = parse_atom(rule_str.trim_end_matches('.'), false)?;
                rules.push(DatalogRule::fact(fact_atom));
            }

            // Reset for next rule
            current_rule.clear();
        }
    }

    Ok(rules)
}

/// Parse a single atom
fn parse_atom(input: &str, negated: bool) -> Result<DatalogAtom> {
    // Extract predicate and arguments
    if let Some(paren_pos) = input.find('(') {
        let predicate = input[..paren_pos].trim();
        let args_str = input[paren_pos + 1..]
            .trim_end_matches('.')
            .trim_end_matches(')');

        let terms: Vec<DatalogTerm> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str
                .split(',')
                .map(|s| parse_term(s.trim()))
                .collect::<Result<Vec<_>>>()?
        };

        let mut atom = DatalogAtom::new(predicate, terms);
        if negated {
            atom.negated = true;
        }
        Ok(atom)
    } else {
        // Atom without arguments
        let mut atom = DatalogAtom::new(input.trim_end_matches('.'), vec![]);
        if negated {
            atom.negated = true;
        }
        Ok(atom)
    }
}

/// Parse a single term (variable or constant)
fn parse_term(input: &str) -> Result<DatalogTerm> {
    let input = input.trim();

    // Variable: starts with uppercase or underscore
    if input.starts_with(|c: char| c.is_uppercase() || c == '_') {
        return Ok(DatalogTerm::Variable(input.to_string()));
    }

    // Constant: try to parse as different types
    // Integer
    if let Ok(i) = input.parse::<i64>() {
        return Ok(DatalogTerm::Constant(Value::Integer(i)));
    }

    // Boolean
    if input == "true" {
        return Ok(DatalogTerm::Constant(Value::Bool(true)));
    }
    if input == "false" {
        return Ok(DatalogTerm::Constant(Value::Bool(false)));
    }

    // String (quoted or unquoted)
    let string_value = input.trim_matches('"').trim_matches('\'');
    Ok(DatalogTerm::Constant(Value::String(Arc::from(
        string_value,
    ))))
}

/// Parse Cedar policies
fn parse_policies(input: &str) -> Result<Vec<Policy>> {
    let mut policies = Vec::new();
    let mut current_policy_id = None;
    let mut policy_content = String::new();

    for line in input.lines() {
        if line.starts_with("permit") || line.starts_with("forbid") {
            // Save previous policy if exists
            if let Some(id) = current_policy_id.take() {
                policies.push(Policy {
                    id,
                    content: policy_content.clone(),
                });
                policy_content.clear();
            }

            // Start new policy
            current_policy_id = Some(format!("policy_{}", policies.len()));
            policy_content.push_str(line);
            policy_content.push('\n');
        } else if current_policy_id.is_some() {
            policy_content.push_str(line);
            policy_content.push('\n');
        }
    }

    // Save last policy
    if let Some(id) = current_policy_id {
        policies.push(Policy {
            id,
            content: policy_content,
        });
    }

    Ok(policies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_atom() {
        let atom = parse_atom("user(X)", false).unwrap();
        assert_eq!(atom.predicate.as_ref(), "user");
        assert_eq!(atom.terms.len(), 1);
        assert!(matches!(atom.terms[0], DatalogTerm::Variable(_)));
    }

    #[test]
    fn test_parse_atom_with_constants() {
        let atom = parse_atom("edge(1, 2)", false).unwrap();
        assert_eq!(atom.predicate.as_ref(), "edge");
        assert_eq!(atom.terms.len(), 2);
        assert!(matches!(
            atom.terms[0],
            DatalogTerm::Constant(Value::Integer(1))
        ));
        assert!(matches!(
            atom.terms[1],
            DatalogTerm::Constant(Value::Integer(2))
        ));
    }

    #[test]
    fn test_parse_negated_atom() {
        let atom = parse_atom("blocked(X)", true).unwrap();
        assert_eq!(atom.predicate.as_ref(), "blocked");
        assert!(atom.negated);
    }

    #[test]
    fn test_parse_rule_with_body() {
        let input = "authorized(X) :- user(X), active(X).";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].head.predicate.as_ref(), "authorized");
        assert_eq!(rules[0].body.len(), 2);
    }

    #[test]
    fn test_parse_fact() {
        let input = "user(alice).";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert!(rules[0].is_fact());
        assert_eq!(rules[0].head.predicate.as_ref(), "user");
    }

    #[test]
    fn test_parse_rule_with_negation() {
        let input = "allowed(X) :- user(X), not blocked(X).";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].body.len(), 2);
        assert!(!rules[0].body[0].negated);
        assert!(rules[0].body[1].negated);
    }

    #[test]
    fn test_parse_term_types() {
        // Integer
        let term = parse_term("42").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::Integer(42))));

        // Boolean
        let term = parse_term("true").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::Bool(true))));

        // String
        let term = parse_term("\"hello\"").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(_))));

        // Variable
        let term = parse_term("X").unwrap();
        assert!(matches!(term, DatalogTerm::Variable(_)));
    }

    #[test]
    fn test_parse_multiple_rules() {
        let input = r#"
            user(alice).
            user(bob).
            admin(alice).
            can_access(U) :- user(U), admin(U).
        "#;
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 4);
        assert!(rules[0].is_fact());
        assert!(rules[1].is_fact());
        assert!(rules[2].is_fact());
        assert!(!rules[3].is_fact());
    }

    // ========== Error Condition Tests ==========

    #[test]
    fn test_parse_rune_file_missing_version() {
        let input = r#"
[data]
key = "value"

[rules]
user(alice).
"#;
        let result = parse_rune_file(input);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), RUNEError::ParseError(msg) if msg.contains("Missing version"))
        );
    }

    #[test]
    fn test_parse_rune_file_invalid_toml() {
        let input = r#"
version = "1.0.0"

[data]
invalid toml here
key = no quotes

[rules]
user(alice).
"#;
        let result = parse_rune_file(input);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), RUNEError::ParseError(msg) if msg.contains("Failed to parse data section"))
        );
    }

    #[test]
    fn test_parse_rune_file_empty_sections() {
        // Test with just version
        let input = r#"version = "1.0.0""#;
        let config = parse_rune_file(input).unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.rules.len(), 0);
        assert_eq!(config.policies.len(), 0);

        // Test with empty sections
        let input = r#"
version = "1.0.0"
[data]
[rules]
[policies]
"#;
        let config = parse_rune_file(input).unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.rules.len(), 0);
        assert_eq!(config.policies.len(), 0);
    }

    #[test]
    fn test_parse_atom_malformed() {
        // Missing closing parenthesis
        let result = parse_atom("user(alice", false);
        assert!(result.is_ok()); // Actually succeeds by trimming

        // Extra closing parenthesis
        let result = parse_atom("user(alice))", false);
        assert!(result.is_ok()); // Succeeds by trimming

        // Empty atom
        let result = parse_atom("", false);
        assert!(result.is_ok()); // Creates atom with empty predicate
        assert_eq!(result.unwrap().predicate.as_ref(), "");

        // Just parentheses
        let result = parse_atom("()", false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().predicate.as_ref(), "");
    }

    #[test]
    fn test_parse_term_edge_cases() {
        // Underscore as variable
        let term = parse_term("_").unwrap();
        assert!(matches!(term, DatalogTerm::Variable(s) if s == "_"));

        // Underscore with name
        let term = parse_term("_Anon").unwrap();
        assert!(matches!(term, DatalogTerm::Variable(s) if s == "_Anon"));

        // Mixed case strings (treated as constants)
        let term = parse_term("camelCase").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(_))));

        // Empty string
        let term = parse_term("").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(s)) if s.as_ref() == ""));

        // Negative numbers actually parse as integers
        let term = parse_term("-42").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::Integer(-42))));

        // Float numbers (treated as string)
        let term = parse_term("3.14").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(s)) if s.as_ref() == "3.14"));

        // Single quotes
        let term = parse_term("'hello'").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(s)) if s.as_ref() == "hello"));

        // Mixed quotes
        let term = parse_term("\"hello'").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(s)) if s.as_ref() == "hello"));
    }

    #[test]
    fn test_split_preserving_parens() {
        // Basic splitting
        let parts = split_preserving_parens("a,b,c");
        assert_eq!(parts, vec!["a", "b", "c"]);

        // With parentheses
        let parts = split_preserving_parens("func(a,b),c");
        assert_eq!(parts, vec!["func(a,b)", "c"]);

        // Nested parentheses
        let parts = split_preserving_parens("func(nested(a,b)),d");
        assert_eq!(parts, vec!["func(nested(a,b))", "d"]);

        // Empty input
        let parts = split_preserving_parens("");
        assert_eq!(parts.len(), 0);

        // Single element
        let parts = split_preserving_parens("single");
        assert_eq!(parts, vec!["single"]);

        // Trailing comma (behavior may vary - check actual implementation)
        let parts = split_preserving_parens("a,b,");
        // The function actually doesn't add an empty element for trailing comma
        assert_eq!(parts, vec!["a", "b"]);

        // Leading comma
        let parts = split_preserving_parens(",a,b");
        assert_eq!(parts, vec!["", "a", "b"]);
    }

    #[test]
    fn test_parse_rules_incomplete() {
        // Rule without period
        let input = "user(alice)";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 0); // Incomplete rule is ignored

        // Rule body without period
        let input = "allowed(X) :- user(X)";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 0);

        // Multi-line incomplete
        let input = r#"
allowed(X) :-
    user(X),
    active(X)
"#;
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_parse_rules_comments_and_empty_lines() {
        let input = r#"
# This is a comment
user(alice).
# Another comment

user(bob).

# Final comment
"#;
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].head.predicate.as_ref(), "user");
        assert_eq!(rules[1].head.predicate.as_ref(), "user");
    }

    #[test]
    fn test_parse_rules_multi_line() {
        let input = r#"
authorized(X) :-
    user(X),
    active(X),
    not blocked(X).
"#;
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].body.len(), 3);
        assert!(!rules[0].body[0].negated);
        assert!(!rules[0].body[1].negated);
        assert!(rules[0].body[2].negated);
    }

    #[test]
    fn test_parse_atom_complex_arguments() {
        // Multiple arguments with mixed types
        let atom = parse_atom("rel(X, 42, \"hello\", true)", false).unwrap();
        assert_eq!(atom.predicate.as_ref(), "rel");
        assert_eq!(atom.terms.len(), 4);

        // Nested function calls (treated as strings)
        let atom = parse_atom("func(nested(X))", false).unwrap();
        assert_eq!(atom.predicate.as_ref(), "func");
        assert_eq!(atom.terms.len(), 1);

        // Empty arguments
        let atom = parse_atom("pred()", false).unwrap();
        assert_eq!(atom.predicate.as_ref(), "pred");
        assert_eq!(atom.terms.len(), 0);
    }

    #[test]
    fn test_parse_policies_basic() {
        let input = r#"
permit (
    principal == User::"alice",
    action == Action::"read",
    resource == File::"data.txt"
);

forbid (
    principal == User::"bob",
    action == Action::"delete",
    resource
);
"#;
        let policies = parse_policies(input).unwrap();
        assert_eq!(policies.len(), 2);
        assert_eq!(policies[0].id, "policy_0");
        assert!(policies[0].content.starts_with("permit"));
        assert_eq!(policies[1].id, "policy_1");
        assert!(policies[1].content.starts_with("forbid"));
    }

    #[test]
    fn test_parse_policies_empty() {
        let input = "";
        let policies = parse_policies(input).unwrap();
        assert_eq!(policies.len(), 0);

        let input = r#"
# Some comment
# No actual policies
"#;
        let policies = parse_policies(input).unwrap();
        assert_eq!(policies.len(), 0);
    }

    #[test]
    fn test_split_sections_edge_cases() {
        // Version with spaces
        let input = r#"version   =   "1.0.0"   "#;
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.version, Some("1.0.0".to_string()));

        // Version without quotes
        let input = r#"version = 1.0.0"#;
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.version, Some("1.0.0".to_string()));

        // Multiple version declarations (takes last)
        let input = r#"
version = "1.0.0"
version = "2.0.0"
"#;
        let sections = split_sections(input).unwrap();
        assert_eq!(sections.version, Some("2.0.0".to_string()));

        // Section without content
        let input = r#"
version = "1.0.0"
[data]
[rules]
[policies]
"#;
        let sections = split_sections(input).unwrap();
        assert!(sections.data.is_none());
        assert!(sections.rules.is_none());
        assert!(sections.policies.is_none());
    }

    #[test]
    fn test_save_section() {
        let mut sections = Sections {
            version: None,
            data: None,
            rules: None,
            policies: None,
        };

        // Save empty content (should do nothing)
        save_section(&mut sections, Some("data"), "");
        assert!(sections.data.is_none());

        // Save actual content
        save_section(&mut sections, Some("data"), "key = value");
        assert_eq!(sections.data, Some("key = value".to_string()));

        // Unknown section (should do nothing)
        save_section(&mut sections, Some("unknown"), "content");

        // None section (should do nothing)
        save_section(&mut sections, None, "content");
    }

    #[test]
    fn test_full_rune_file_parsing() {
        let input = r#"
version = "1.0.0"

[data]
app_name = "test-app"
debug = true
port = 8080

[rules]
# Facts
user(alice).
user(bob).
admin(alice).

# Rules
can_access(U) :- user(U), admin(U).
authorized(U, R) :-
    user(U),
    resource(R),
    not blocked(U, R).

[policies]
permit (
    principal == User::"alice",
    action == Action::"read",
    resource
);
"#;
        let config = parse_rune_file(input).unwrap();

        assert_eq!(config.version, "1.0.0");

        // Check data section
        assert!(config.data.get("app_name").is_some());
        assert!(config.data.get("debug").is_some());
        assert!(config.data.get("port").is_some());

        // Check rules
        assert_eq!(config.rules.len(), 5);
        assert!(config.rules[0].is_fact());
        assert!(config.rules[1].is_fact());
        assert!(config.rules[2].is_fact());
        assert!(!config.rules[3].is_fact());
        assert!(!config.rules[4].is_fact());

        // Check policies
        assert_eq!(config.policies.len(), 1);
        assert_eq!(config.policies[0].id, "policy_0");
    }

    #[test]
    fn test_parse_rules_with_special_characters() {
        // Rule with underscores in predicate
        let input = "is_valid_user(X) :- user(X), active(X).";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].head.predicate.as_ref(), "is_valid_user");

        // Rule with numbers in predicate
        let input = "rule2(X) :- pred1(X).";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].head.predicate.as_ref(), "rule2");
    }

    #[test]
    fn test_parse_term_with_large_numbers() {
        // Max i64
        let term = parse_term("9223372036854775807").unwrap();
        assert!(matches!(
            term,
            DatalogTerm::Constant(Value::Integer(9223372036854775807))
        ));

        // Number too large for i64 (becomes string)
        let term = parse_term("99999999999999999999").unwrap();
        assert!(matches!(term, DatalogTerm::Constant(Value::String(_))));
    }
}
