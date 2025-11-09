//! Parser for RUNE configuration files

use crate::error::{RUNEError, Result};
use serde::{Deserialize, Serialize};

/// Parsed RUNE configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RUNEConfig {
    /// Version string
    pub version: String,
    /// Data section (TOML-style)
    pub data: toml::Value,
    /// Datalog rules
    pub rules: Vec<Rule>,
    /// Cedar policies
    pub policies: Vec<Policy>,
}

/// A Datalog rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Head of the rule (conclusion)
    pub head: Atom,
    /// Body of the rule (conditions)
    pub body: Vec<Atom>,
}

/// An atom in a Datalog rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    /// Predicate name
    pub predicate: String,
    /// Arguments
    pub args: Vec<Term>,
}

/// A term in an atom
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Term {
    /// Variable (starts with uppercase)
    Variable(String),
    /// Constant value
    Constant(String),
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

/// Parse Datalog rules
fn parse_rules(input: &str) -> Result<Vec<Rule>> {
    // Simplified parser - full implementation would use nom combinators
    let mut rules = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Simple rule parsing (head :- body)
        if let Some((head, body)) = line.split_once(":-") {
            let head_atom = parse_atom(head.trim())?;
            let body_atoms = body
                .split(',')
                .map(|s| parse_atom(s.trim()))
                .collect::<Result<Vec<_>>>()?;

            rules.push(Rule {
                head: head_atom,
                body: body_atoms,
            });
        }
    }

    Ok(rules)
}

/// Parse a single atom
fn parse_atom(input: &str) -> Result<Atom> {
    // Extract predicate and arguments
    if let Some(paren_pos) = input.find('(') {
        let predicate = input[..paren_pos].trim().to_string();
        let args_str = input[paren_pos + 1..]
            .trim_end_matches(')')
            .trim_end_matches('.');

        let args = args_str
            .split(',')
            .map(|s| {
                let s = s.trim();
                if s.chars().next().map_or(false, |c| c.is_uppercase()) {
                    Term::Variable(s.to_string())
                } else {
                    Term::Constant(s.trim_matches('"').to_string())
                }
            })
            .collect();

        Ok(Atom { predicate, args })
    } else {
        // Atom without arguments
        Ok(Atom {
            predicate: input.trim_end_matches('.').to_string(),
            args: Vec::new(),
        })
    }
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
    fn test_parse_simple_rule() {
        let atom = parse_atom("user(X)").unwrap();
        assert_eq!(atom.predicate, "user");
        assert_eq!(atom.args.len(), 1);
    }

    #[test]
    fn test_parse_rule_with_body() {
        let input = "authorized(X) :- user(X), active(X).";
        let rules = parse_rules(input).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].head.predicate, "authorized");
        assert_eq!(rules[0].body.len(), 2);
    }
}
