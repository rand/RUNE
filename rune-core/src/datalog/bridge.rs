//! Bridge between Cedar entities and Datalog facts
//!
//! Provides bidirectional conversion between Cedar authorization primitives
//! (Principal, Resource, Action) and Datalog facts for rule evaluation.
//!
//! Design principles:
//! - Flat fact representation for easy querying
//! - Preserve entity hierarchies
//! - Handle attributes and parameters
//! - Bidirectional conversion with roundtrip preservation
//! - Efficient sync mechanism for updates
//!
//! ## Conversion Patterns
//!
//! **Cedar → Datalog:**
//! - Entity(id, type, attrs) → entity(id, type) + entity_attr(id, key, val)
//! - Hierarchy → entity_parent(child, parent) facts
//!
//! **Datalog → Cedar:**
//! - Collect facts by predicate and entity ID
//! - Reconstruct entity hierarchies from parent facts
//! - Merge attributes from attribute facts

use crate::facts::Fact;
use crate::request::Request;
use crate::types::{Action, Entity, Principal, Resource, Value};
use std::collections::HashMap;
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

    // ==================== BIDIRECTIONAL CONVERSION ====================
    // Methods for converting Datalog facts back to Cedar entities

    /// Reconstruct an Entity from Datalog facts
    ///
    /// Collects all facts for a given entity ID and prefix, rebuilding:
    /// - Entity identity (from entity(id, type))
    /// - Attributes (from entity_attr(id, key, val))
    /// - Parent hierarchy (from entity_parent(id, parent))
    pub fn facts_to_entity(facts: &[Fact], entity_id: &str, prefix: &str) -> Option<Entity> {
        // Find identity fact: entity(id, type)
        let identity_fact = facts
            .iter()
            .find(|f| f.predicate.as_ref() == prefix && Self::fact_has_id(&f.args, entity_id))?;

        // Extract entity type (second argument)
        let entity_type = match &identity_fact.args.get(1)? {
            Value::String(s) => s.clone(),
            _ => return None,
        };

        // Create base entity
        let mut entity = Entity::new(entity_type.as_ref(), entity_id);

        // Collect attributes from entity_attr(id, key, val) facts
        let attr_predicate = format!("{}_attr", prefix);
        for fact in facts
            .iter()
            .filter(|f| f.predicate.as_ref() == attr_predicate)
        {
            if Self::fact_has_id(&fact.args, entity_id) {
                if let (Some(Value::String(key)), Some(value)) =
                    (fact.args.get(1), fact.args.get(2))
                {
                    entity = entity.with_attribute(key.as_ref(), value.clone());
                }
            }
        }

        // Collect parent entities from entity_parent(id, parent_id) facts
        let parent_predicate = format!("{}_parent", prefix);
        for fact in facts
            .iter()
            .filter(|f| f.predicate.as_ref() == parent_predicate)
        {
            if Self::fact_has_id(&fact.args, entity_id) {
                if let Some(Value::String(parent_id)) = fact.args.get(1) {
                    // Recursively reconstruct parent
                    if let Some(parent) = Self::facts_to_entity(facts, parent_id.as_ref(), prefix) {
                        entity = entity.with_parent(parent);
                    }
                }
            }
        }

        Some(entity)
    }

    /// Helper: Check if a fact's arguments contain a specific entity ID
    fn fact_has_id(args: &[Value], id: &str) -> bool {
        args.first()
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_ref() == id),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// Reconstruct a Principal from Datalog facts
    ///
    /// Finds all facts with the "principal" prefix and reconstructs the entity
    pub fn facts_to_principal(facts: &[Fact], principal_id: &str) -> Option<Principal> {
        let entity = Self::facts_to_entity(facts, principal_id, "principal")?;
        Some(Principal { entity })
    }

    /// Reconstruct a Resource from Datalog facts
    ///
    /// Finds all facts with the "resource" prefix and reconstructs the entity
    pub fn facts_to_resource(facts: &[Fact], resource_id: &str) -> Option<Resource> {
        let entity = Self::facts_to_entity(facts, resource_id, "resource")?;
        Some(Resource { entity })
    }

    /// Reconstruct an Action from Datalog facts
    ///
    /// Collects action(name) and action_param(name, key, val) facts
    pub fn facts_to_action(facts: &[Fact], action_name: &str) -> Option<Action> {
        // Find action identity fact
        facts.iter().find(|f| {
            f.predicate.as_ref() == "action"
                && f.args
                    .first()
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_ref() == action_name),
                        _ => None,
                    })
                    .unwrap_or(false)
        })?;

        // Create base action
        let mut action = Action::new(action_name);

        // Collect parameters from action_param(name, key, val) facts
        for fact in facts
            .iter()
            .filter(|f| f.predicate.as_ref() == "action_param")
        {
            if let Some(Value::String(name)) = fact.args.first() {
                if name.as_ref() == action_name {
                    if let (Some(Value::String(key)), Some(value)) =
                        (fact.args.get(1), fact.args.get(2))
                    {
                        action = action.with_parameter(key.as_ref(), value.clone());
                    }
                }
            }
        }

        Some(action)
    }

    /// Reconstruct a complete Request from Datalog facts
    ///
    /// Uses request_principal, request_action, and request_resource metadata facts
    /// to identify the components, then reconstructs each one
    pub fn facts_to_request(facts: &[Fact]) -> Option<Request> {
        // Find principal ID from request_principal(id) fact
        let principal_id = facts
            .iter()
            .find(|f| f.predicate.as_ref() == "request_principal")
            .and_then(|f| f.args.first())
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })?;

        // Find action name from request_action(name) fact
        let action_name = facts
            .iter()
            .find(|f| f.predicate.as_ref() == "request_action")
            .and_then(|f| f.args.first())
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })?;

        // Find resource ID from request_resource(id) fact
        let resource_id = facts
            .iter()
            .find(|f| f.predicate.as_ref() == "request_resource")
            .and_then(|f| f.args.first())
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })?;

        // Reconstruct each component
        let principal = Self::facts_to_principal(facts, principal_id.as_ref())?;
        let action = Self::facts_to_action(facts, action_name.as_ref())?;
        let resource = Self::facts_to_resource(facts, resource_id.as_ref())?;

        // Reconstruct context from context(key, val) facts
        let mut request = Request::new(principal, action, resource);
        for fact in facts.iter().filter(|f| f.predicate.as_ref() == "context") {
            if let (Some(Value::String(key)), Some(value)) = (fact.args.get(0), fact.args.get(1)) {
                request = request.with_context(key.as_ref(), value.clone());
            }
        }

        Some(request)
    }

    /// Extract entities from derived facts (query results)
    ///
    /// Useful for mapping Datalog query results back to Cedar entities.
    /// Groups facts by entity ID and reconstructs each unique entity.
    pub fn extract_entities_from_facts(facts: &[Fact], prefix: &str) -> Vec<Entity> {
        // Group facts by entity ID
        let mut entity_ids: HashMap<Arc<str>, Vec<&Fact>> = HashMap::new();

        for fact in facts {
            if fact.predicate.as_ref() == prefix
                || fact.predicate.starts_with(&format!("{}_", prefix))
            {
                if let Some(Value::String(id)) = fact.args.first() {
                    entity_ids
                        .entry(id.clone())
                        .or_insert_with(Vec::new)
                        .push(fact);
                }
            }
        }

        // Reconstruct each entity
        entity_ids
            .into_keys()
            .filter_map(|id| {
                // Collect all facts for this entity ID
                let entity_facts: Vec<Fact> = facts
                    .iter()
                    .filter(|f| {
                        f.predicate.as_ref() == prefix
                            || f.predicate.starts_with(&format!("{}_", prefix))
                    })
                    .cloned()
                    .collect();

                Self::facts_to_entity(&entity_facts, id.as_ref(), prefix)
            })
            .collect()
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

    // ==================== BIDIRECTIONAL CONVERSION TESTS ====================

    #[test]
    fn test_roundtrip_principal() {
        // Create original principal with attributes
        let original = Principal::new("User", "alice")
            .entity
            .with_attribute("role", Value::string("admin"))
            .with_attribute("department", Value::string("engineering"));

        let principal = Principal { entity: original };

        // Convert to facts
        let facts = CedarDatalogBridge::principal_to_facts(&principal);

        // Convert back to principal (use actual ID, not entity_type::id format)
        let reconstructed = CedarDatalogBridge::facts_to_principal(&facts, &principal.entity.id)
            .expect("Failed to reconstruct principal");

        // Verify identity
        assert_eq!(reconstructed.entity.id, principal.entity.id);
        assert_eq!(
            reconstructed.entity.entity_type,
            principal.entity.entity_type
        );

        // Verify attributes
        assert_eq!(
            reconstructed.entity.attributes.get("role"),
            Some(&Value::string("admin"))
        );
        assert_eq!(
            reconstructed.entity.attributes.get("department"),
            Some(&Value::string("engineering"))
        );
    }

    #[test]
    fn test_roundtrip_resource() {
        // Create original resource
        let original = Resource::new("File", "/tmp/secret.txt")
            .entity
            .with_attribute("owner", Value::string("alice"))
            .with_attribute("confidential", Value::Bool(true));

        let resource = Resource { entity: original };

        // Convert to facts
        let facts = CedarDatalogBridge::resource_to_facts(&resource);

        // Convert back (use actual ID)
        let reconstructed = CedarDatalogBridge::facts_to_resource(&facts, &resource.entity.id)
            .expect("Failed to reconstruct resource");

        // Verify
        assert_eq!(reconstructed.entity.id, resource.entity.id);
        assert_eq!(
            reconstructed.entity.attributes.get("owner"),
            Some(&Value::string("alice"))
        );
        assert_eq!(
            reconstructed.entity.attributes.get("confidential"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn test_roundtrip_action() {
        // Create original action
        let original = Action::new("file:read")
            .with_parameter("mode", Value::string("readonly"))
            .with_parameter("level", Value::Integer(5));

        // Convert to facts
        let facts = CedarDatalogBridge::action_to_facts(&original);

        // Convert back
        let reconstructed = CedarDatalogBridge::facts_to_action(&facts, "file:read")
            .expect("Failed to reconstruct action");

        // Verify
        assert_eq!(reconstructed.name, original.name);
        assert_eq!(
            reconstructed.parameters.get("mode"),
            Some(&Value::string("readonly"))
        );
        assert_eq!(
            reconstructed.parameters.get("level"),
            Some(&Value::Integer(5))
        );
    }

    #[test]
    fn test_roundtrip_request() {
        // Create original request
        let principal = Principal::user("alice")
            .entity
            .with_attribute("role", Value::string("developer"));
        let action = Action::new("read").with_parameter("urgent", Value::Bool(true));
        let resource = Resource::file("/data/config.json")
            .entity
            .with_attribute("classification", Value::string("public"));

        let original = Request::new(
            Principal { entity: principal },
            action,
            Resource { entity: resource },
        )
        .with_context("time", Value::string("2024-01-15"))
        .with_context("ip", Value::string("192.168.1.1"));

        // Convert to facts (both entity facts and metadata facts)
        let mut facts = CedarDatalogBridge::request_to_facts(&original);
        facts.extend(CedarDatalogBridge::request_metadata_facts(&original));

        // Convert back
        let reconstructed =
            CedarDatalogBridge::facts_to_request(&facts).expect("Failed to reconstruct request");

        // Verify principal
        assert_eq!(
            reconstructed.principal.entity.id,
            original.principal.entity.id
        );
        assert_eq!(
            reconstructed.principal.entity.attributes.get("role"),
            Some(&Value::string("developer"))
        );

        // Verify action
        assert_eq!(reconstructed.action.name, original.action.name);
        assert_eq!(
            reconstructed.action.parameters.get("urgent"),
            Some(&Value::Bool(true))
        );

        // Verify resource
        assert_eq!(
            reconstructed.resource.entity.id,
            original.resource.entity.id
        );

        // Verify context
        assert_eq!(
            reconstructed.context.get("time"),
            Some(&Value::string("2024-01-15"))
        );
    }

    #[test]
    fn test_roundtrip_hierarchical_entity() {
        // Create entity with parent hierarchy
        let parent = Entity::new("Group", "admins")
            .with_attribute("level", Value::Integer(10))
            .with_attribute("department", Value::string("IT"));

        let principal = Principal::user("alice")
            .entity
            .with_attribute("role", Value::string("developer"))
            .with_parent(parent);

        let principal = Principal { entity: principal };

        // Convert to facts
        let facts = CedarDatalogBridge::principal_to_facts(&principal);

        // Convert back (use actual ID)
        let reconstructed = CedarDatalogBridge::facts_to_principal(&facts, &principal.entity.id)
            .expect("Failed to reconstruct principal with hierarchy");

        // Verify base entity
        assert_eq!(reconstructed.entity.id, principal.entity.id);
        assert_eq!(
            reconstructed.entity.attributes.get("role"),
            Some(&Value::string("developer"))
        );

        // Verify parent hierarchy
        assert_eq!(reconstructed.entity.parents.len(), 1);
        let parent = &reconstructed.entity.parents[0];
        // Check parent ID contains "admins" (actual ID format may vary)
        assert!(parent.id.as_ref().contains("admins"));
        assert_eq!(parent.attributes.get("level"), Some(&Value::Integer(10)));
    }

    #[test]
    fn test_extract_entities_from_facts() {
        // Create multiple principals
        let alice = Principal::user("alice")
            .entity
            .with_attribute("role", Value::string("admin"));
        let bob = Principal::user("bob")
            .entity
            .with_attribute("role", Value::string("user"));

        // Convert to facts
        let mut facts = CedarDatalogBridge::principal_to_facts(&Principal { entity: alice });
        facts.extend(CedarDatalogBridge::principal_to_facts(&Principal {
            entity: bob,
        }));

        // Extract all principals
        let entities = CedarDatalogBridge::extract_entities_from_facts(&facts, "principal");

        // Should extract both entities
        assert_eq!(entities.len(), 2);

        // Verify both entities are present (check just the ID part, not entity_type::id)
        let ids: Vec<_> = entities.iter().map(|e| e.id.as_ref()).collect();
        // IDs should contain alice and bob (Principal::user creates these IDs)
        assert!(ids.iter().any(|id| id.contains("alice")));
        assert!(ids.iter().any(|id| id.contains("bob")));
    }

    #[test]
    fn test_facts_to_entity_missing() {
        let facts = vec![Fact::new(
            "principal".to_string(),
            vec![Value::string("User::alice"), Value::string("User")],
        )];

        // Try to reconstruct non-existent entity
        let result = CedarDatalogBridge::facts_to_entity(&facts, "User::bob", "principal");

        // Should return None
        assert!(result.is_none());
    }

    #[test]
    fn test_facts_to_request_missing_metadata() {
        // Create facts without metadata
        let principal = Principal::user("alice");
        let facts = CedarDatalogBridge::principal_to_facts(&principal);

        // Try to reconstruct request (should fail without metadata)
        let result = CedarDatalogBridge::facts_to_request(&facts);

        // Should return None because metadata facts are missing
        assert!(result.is_none());
    }
}
