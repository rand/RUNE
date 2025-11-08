//! Cedar policy integration

use crate::engine::{AuthorizationResult, Decision};
use crate::error::{Result, RUNEError};
use crate::request::Request;
use cedar_policy::{Authorizer, Context, Entities, PolicySet as CedarPolicySet, Request as CedarRequest};
use cedar_policy::{Entity as CedarEntity, EntityId, EntityTypeName, EntityUid};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

/// Policy set wrapper for Cedar
pub struct PolicySet {
    cedar_policies: CedarPolicySet,
    authorizer: Authorizer,
}

impl PolicySet {
    /// Create a new empty policy set
    pub fn new() -> Self {
        PolicySet {
            cedar_policies: CedarPolicySet::new(),
            authorizer: Authorizer::new(),
        }
    }

    /// Load policies from a string
    pub fn load_policies(&mut self, policy_str: &str) -> Result<()> {
        let policies = policy_str.parse::<CedarPolicySet>()
            .map_err(|e| RUNEError::ConfigError(format!("Failed to parse policies: {}", e)))?;

        self.cedar_policies = policies;
        Ok(())
    }

    /// Add a single policy
    pub fn add_policy(&mut self, _id: &str, policy_str: &str) -> Result<()> {
        use cedar_policy::Policy;

        // Parse policy with a template-linked ID
        let policy = Policy::parse(None, policy_str)
            .map_err(|e| RUNEError::ConfigError(format!("Failed to parse policy: {}", e)))?;

        // For Cedar 3.x, we need to rebuild the policy set
        let mut new_set = CedarPolicySet::new();
        new_set.add(policy)
            .map_err(|e| RUNEError::ConfigError(format!("Failed to add policy: {}", e)))?;

        // Merge with existing policies
        for p in self.cedar_policies.policies() {
            new_set.add(p.clone())
                .map_err(|e| RUNEError::ConfigError(format!("Failed to merge policy: {}", e)))?;
        }

        self.cedar_policies = new_set;
        Ok(())
    }

    /// Evaluate a request against the policies
    pub fn evaluate(&self, request: &Request) -> Result<AuthorizationResult> {
        let start = Instant::now();

        // Convert RUNE request to Cedar request
        let cedar_request = self.convert_request(request)?;

        // Create entities from the request
        let entities = self.create_entities(request)?;

        // Evaluate with Cedar
        let response = self.authorizer.is_authorized(
            &cedar_request,
            &self.cedar_policies,
            &entities,
        );

        // Convert Cedar decision to RUNE decision
        let decision = match response.decision() {
            cedar_policy::Decision::Allow => Decision::Permit,
            cedar_policy::Decision::Deny => Decision::Deny,
        };

        // Collect diagnostics
        let mut evaluated_rules = Vec::new();
        let mut explanation = String::new();

        // Collect any errors
        for error in response.diagnostics().errors() {
            explanation.push_str(&format!("Error: {}; ", error));
        }

        // Collect the policy IDs that contributed to the decision
        for policy_id in response.diagnostics().reason() {
            evaluated_rules.push(policy_id.to_string());
        }

        if explanation.is_empty() {
            explanation = match decision {
                Decision::Permit => "Permitted by Cedar policies".to_string(),
                Decision::Deny => "Denied by Cedar policies".to_string(),
                Decision::Forbid => "Forbidden by Cedar policies".to_string(),
            };
        }

        Ok(AuthorizationResult {
            decision,
            explanation,
            evaluated_rules,
            facts_used: vec![], // Cedar doesn't expose this directly
            evaluation_time_ns: start.elapsed().as_nanos() as u64,
            cached: false,
        })
    }

    /// Convert RUNE request to Cedar request
    fn convert_request(&self, request: &Request) -> Result<CedarRequest> {
        // Convert principal
        let principal_type = EntityTypeName::from_str(request.principal.entity.entity_type.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid principal type: {}", e)))?;

        let principal_id = EntityId::from_str(request.principal.entity.id.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid principal ID: {}", e)))?;

        let principal = EntityUid::from_type_name_and_id(principal_type, principal_id);

        // Convert action
        let action_type = EntityTypeName::from_str("Action")
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid action type: {}", e)))?;

        let action_id = EntityId::from_str(request.action.name.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid action ID: {}", e)))?;

        let action = EntityUid::from_type_name_and_id(action_type, action_id);

        // Convert resource
        let resource_type = EntityTypeName::from_str(request.resource.entity.entity_type.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid resource type: {}", e)))?;

        let resource_id = EntityId::from_str(request.resource.entity.id.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid resource ID: {}", e)))?;

        let resource = EntityUid::from_type_name_and_id(resource_type, resource_id);

        // Create context (simplified for now)
        let context = Context::empty();

        Ok(CedarRequest::new(
            Some(principal),
            Some(action),
            Some(resource),
            context,
            None,
        ).map_err(|e| RUNEError::InvalidRequest(format!("Failed to create Cedar request: {}", e)))?)
    }

    /// Create entities for Cedar evaluation
    fn create_entities(&self, request: &Request) -> Result<Entities> {
        // Collect all entities first
        let mut all_entities = Vec::new();

        // Add principal entity
        let principal_entity = self.convert_entity(&request.principal.entity)?;
        all_entities.push(principal_entity);

        // Add resource entity
        let resource_entity = self.convert_entity(&request.resource.entity)?;
        all_entities.push(resource_entity);

        // Add action entity
        let action_type = EntityTypeName::from_str("Action")
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid action type: {}", e)))?;

        let action_id = EntityId::from_str(request.action.name.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid action ID: {}", e)))?;

        let action_uid = EntityUid::from_type_name_and_id(action_type, action_id);

        // Create action entity with empty attributes for now
        let action_entity = CedarEntity::new(
            action_uid,
            HashMap::new(),
            std::collections::HashSet::new(),
        ).map_err(|e| RUNEError::InvalidRequest(format!("Failed to create action entity: {}", e)))?;

        all_entities.push(action_entity);

        // Create entities using from_entities which takes ownership properly
        Entities::from_entities(all_entities, None)
            .map_err(|e| RUNEError::InvalidRequest(format!("Failed to create entities: {}", e)))
    }

    /// Convert RUNE entity to Cedar entity
    fn convert_entity(&self, entity: &crate::types::Entity) -> Result<CedarEntity> {
        let entity_type = EntityTypeName::from_str(entity.entity_type.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid entity type: {}", e)))?;

        let entity_id = EntityId::from_str(entity.id.as_ref())
            .map_err(|e| RUNEError::InvalidRequest(format!("Invalid entity ID: {}", e)))?;

        let uid = EntityUid::from_type_name_and_id(entity_type, entity_id);

        // Convert attributes (simplified for now - Cedar has strict typing)
        let attributes = HashMap::new();

        // Convert parent relationships
        let mut parents = std::collections::HashSet::new();
        for parent in &entity.parents {
            let parent_type = EntityTypeName::from_str(parent.entity_type.as_ref())
                .map_err(|e| RUNEError::InvalidRequest(format!("Invalid parent type: {}", e)))?;

            let parent_id = EntityId::from_str(parent.id.as_ref())
                .map_err(|e| RUNEError::InvalidRequest(format!("Invalid parent ID: {}", e)))?;

            let parent_uid = EntityUid::from_type_name_and_id(parent_type, parent_id);
            parents.insert(parent_uid);
        }

        CedarEntity::new(uid, attributes, parents)
            .map_err(|e| RUNEError::InvalidRequest(format!("Failed to create entity: {}", e)))
    }
}

impl Default for PolicySet {
    fn default() -> Self {
        Self::new()
    }
}