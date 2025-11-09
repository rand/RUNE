//! Bridge between Cedar entities and Datalog facts
//!
//! Converts Cedar authorization primitives (Principal, Resource, Action)
//! into Datalog facts that can be used in rule evaluation.
//!
//! Design principles:
//! - Flat fact representation for easy querying
//! - Preserve entity hierarchies
//! - Handle attributes and parameters
//! - Enable bidirectional conversion (future)

use crate::facts::Fact;
use crate::request::Request;
use crate::types::{Action, Entity, Principal, Resource, Value};
use std::sync::Arc;

/// Bridge for converting Cedar entities to Datalog facts
pub struct CedarDatalogBridge;

impl CedarDatalogBridge {
    /// Convert a Request to Datalog facts
    ///
    /// Generates facts for:
    /// - Principal entity and attributes
    /// - Action and parameters
    /// - Resource entity and attributes
    /// - Request context
    pub fn request_to_facts(request: &Request) -> Vec<Fact> {
        let mut facts = Vec::new();

        // Principal facts
        facts.extend(Self::principal_to_facts(&request.principal));

        // Action facts
        facts.extend(Self::action_to_facts(&request.action));

        // Resource facts
        facts.extend(Self::resource_to_facts(&request.resource));

        // Context facts
        for (key, value) in request.context.iter() {
            facts.push(Fact::new(
                "context".to_string(),
                vec![Value::String(Arc::from(key.as_str())), value.clone()],
            ));
        }

        facts
    }

    /// Convert a Principal to Datalog facts
    ///
    /// Generates:
    /// - `principal(id, type)` - Principal identity
    /// - `principal_attr(id, key, value)` - Principal attributes
    /// - `principal_parent(id, parent_id)` - Hierarchical relationships
    pub fn principal_to_facts(principal: &Principal) -> Vec<Fact> {
        let mut facts = Vec::new();
        let entity = &principal.entity;

        // Principal identity fact
        facts.push(Fact::new(
            "principal".to_string(),
            vec![
                Value::String(entity.id.clone()),
                Value::String(entity.entity_type.clone()),
            ],
        ));

        // Principal attribute facts
        for (key, value) in entity.attributes.iter() {
            facts.push(Fact::new(
                "principal_attr".to_string(),
                vec![
                    Value::String(entity.id.clone()),
                    Value::String(Arc::from(key.as_str())),
                    value.clone(),
                ],
            ));
        }

        // Principal parent facts (hierarchical)
        for parent in &entity.parents {
            facts.push(Fact::new(
                "principal_parent".to_string(),
                vec![
                    Value::String(entity.id.clone()),
                    Value::String(parent.id.clone()),
                ],
            ));

            // Recursively add parent facts
            facts.extend(Self::entity_to_facts(parent, "principal"));
        }

        facts
    }

    /// Convert a Resource to Datalog facts
    ///
    /// Generates:
    /// - `resource(id, type)` - Resource identity
    /// - `resource_attr(id, key, value)` - Resource attributes
    /// - `resource_parent(id, parent_id)` - Hierarchical relationships
    pub fn resource_to_facts(resource: &Resource) -> Vec<Fact> {
        let mut facts = Vec::new();
        let entity = &resource.entity;

        // Resource identity fact
        facts.push(Fact::new(
            "resource".to_string(),
            vec![
                Value::String(entity.id.clone()),
                Value::String(entity.entity_type.clone()),
            ],
        ));

        // Resource attribute facts
        for (key, value) in entity.attributes.iter() {
            facts.push(Fact::new(
                "resource_attr".to_string(),
                vec![
                    Value::String(entity.id.clone()),
                    Value::String(Arc::from(key.as_str())),
                    value.clone(),
                ],
            ));
        }

        // Resource parent facts (hierarchical)
        for parent in &entity.parents {
            facts.push(Fact::new(
                "resource_parent".to_string(),
                vec![
                    Value::String(entity.id.clone()),
                    Value::String(parent.id.clone()),
                ],
            ));

            // Recursively add parent facts
            facts.extend(Self::entity_to_facts(parent, "resource"));
        }

        facts
    }

    /// Convert an Action to Datalog facts
    ///
    /// Generates:
    /// - `action(name)` - Action identity
    /// - `action_param(name, key, value)` - Action parameters
    pub fn action_to_facts(action: &Action) -> Vec<Fact> {
        let mut facts = Vec::new();

        // Action identity fact
        facts.push(Fact::new(
            "action".to_string(),
            vec![Value::String(action.name.clone())],
        ));

        // Action parameter facts
        for (key, value) in action.parameters.iter() {
            facts.push(Fact::new(
                "action_param".to_string(),
                vec![
                    Value::String(action.name.clone()),
                    Value::String(Arc::from(key.as_str())),
                    value.clone(),
                ],
            ));
        }

        facts
    }

    /// Convert a generic Entity to Datalog facts
    ///
    /// Used for hierarchical entities (parents)
    fn entity_to_facts(entity: &Entity, prefix: &str) -> Vec<Fact> {
        let mut facts = Vec::new();

        // Entity identity
        facts.push(Fact::new(
            prefix.to_string(),
            vec![
                Value::String(entity.id.clone()),
                Value::String(entity.entity_type.clone()),
            ],
        ));

        // Entity attributes
        for (key, value) in entity.attributes.iter() {
            facts.push(Fact::new(
                format!("{}_attr", prefix),
                vec![
                    Value::String(entity.id.clone()),
                    Value::String(Arc::from(key.as_str())),
                    value.clone(),
                ],
            ));
        }

        // Recursively handle parent hierarchy
        for parent in &entity.parents {
            facts.push(Fact::new(
                format!("{}_parent", prefix),
                vec![
                    Value::String(entity.id.clone()),
                    Value::String(parent.id.clone()),
                ],
            ));

            facts.extend(Self::entity_to_facts(parent, prefix));
        }

        facts
    }

    /// Create Datalog facts for common authorization patterns
    ///
    /// Generates derived facts for:
    /// - `request_principal(id)` - The principal making this request
    /// - `request_action(name)` - The action being performed
    /// - `request_resource(id)` - The resource being accessed
    pub fn request_metadata_facts(request: &Request) -> Vec<Fact> {
        vec![
            Fact::new(
                "request_principal".to_string(),
                vec![Value::String(request.principal.entity.id.clone())],
            ),
            Fact::new(
                "request_action".to_string(),
                vec![Value::String(request.action.name.clone())],
            ),
            Fact::new(
                "request_resource".to_string(),
                vec![Value::String(request.resource.entity.id.clone())],
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    #[test]
    fn test_principal_to_facts() {
        let principal = Principal::new("User", "alice")
            .entity
            .with_attribute("role", Value::string("admin"))
            .with_attribute("department", Value::string("engineering"));

        let principal = Principal { entity: principal };

        let facts = CedarDatalogBridge::principal_to_facts(&principal);

        // Should have: 1 identity + 2 attributes = 3 facts
        assert_eq!(facts.len(), 3);

        // Check principal identity fact
        let identity_fact = facts.iter().find(|f| f.predicate.as_ref() == "principal");
        assert!(identity_fact.is_some());

        // Check attribute facts
        let attr_facts: Vec<_> = facts
            .iter()
            .filter(|f| f.predicate.as_ref() == "principal_attr")
            .collect();
        assert_eq!(attr_facts.len(), 2);
    }

    #[test]
    fn test_resource_to_facts() {
        let resource = Resource::new("File", "/tmp/secret.txt")
            .entity
            .with_attribute("owner", Value::string("alice"))
            .with_attribute("confidential", Value::Bool(true));

        let resource = Resource { entity: resource };

        let facts = CedarDatalogBridge::resource_to_facts(&resource);

        // Should have: 1 identity + 2 attributes = 3 facts
        assert_eq!(facts.len(), 3);

        // Check resource identity
        let identity_fact = facts.iter().find(|f| f.predicate.as_ref() == "resource");
        assert!(identity_fact.is_some());
    }

    #[test]
    fn test_action_to_facts() {
        let action = Action::new("file:read").with_parameter("mode", Value::string("readonly"));

        let facts = CedarDatalogBridge::action_to_facts(&action);

        // Should have: 1 identity + 1 parameter = 2 facts
        assert_eq!(facts.len(), 2);

        // Check action identity
        let identity_fact = facts.iter().find(|f| f.predicate.as_ref() == "action");
        assert!(identity_fact.is_some());
    }

    #[test]
    fn test_request_to_facts() {
        let principal = Principal::user("alice");
        let action = Action::new("read");
        let resource = Resource::file("/tmp/data.txt");

        let request = Request::new(principal, action, resource);

        let facts = CedarDatalogBridge::request_to_facts(&request);

        // Should have principal, action, and resource facts
        assert!(!facts.is_empty());

        // Check for each fact type
        assert!(facts.iter().any(|f| f.predicate.as_ref() == "principal"));
        assert!(facts.iter().any(|f| f.predicate.as_ref() == "action"));
        assert!(facts.iter().any(|f| f.predicate.as_ref() == "resource"));
    }

    #[test]
    fn test_hierarchical_entities() {
        let parent = Entity::new("Group", "admins").with_attribute("level", Value::Integer(10));

        let principal = Principal::user("alice").entity.with_parent(parent);

        let principal = Principal { entity: principal };

        let facts = CedarDatalogBridge::principal_to_facts(&principal);

        // Should have principal facts + parent relationship + parent facts
        assert!(facts.len() > 1);

        // Check for parent relationship fact
        let parent_fact = facts
            .iter()
            .find(|f| f.predicate.as_ref() == "principal_parent");
        assert!(parent_fact.is_some());

        // Check that parent facts were generated
        let parent_identity = facts
            .iter()
            .filter(|f| f.predicate.as_ref() == "principal")
            .count();
        assert_eq!(parent_identity, 2); // alice + admins
    }

    #[test]
    fn test_request_metadata_facts() {
        let principal = Principal::agent("agent-007");
        let action = Action::new("execute");
        let resource = Resource::api("/v1/endpoint");

        let request = Request::new(principal, action, resource);

        let metadata = CedarDatalogBridge::request_metadata_facts(&request);

        assert_eq!(metadata.len(), 3);
        assert!(metadata
            .iter()
            .any(|f| f.predicate.as_ref() == "request_principal"));
        assert!(metadata
            .iter()
            .any(|f| f.predicate.as_ref() == "request_action"));
        assert!(metadata
            .iter()
            .any(|f| f.predicate.as_ref() == "request_resource"));
    }
}
