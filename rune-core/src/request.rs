//! Request types for authorization

use crate::types::{Action, Principal, Resource, Value};
use ahash::AHasher;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Authorization request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    /// Principal making the request
    pub principal: Principal,
    /// Action being performed
    pub action: Action,
    /// Resource being accessed
    pub resource: Resource,
    /// Additional context
    pub context: Arc<BTreeMap<String, Value>>,
    /// Request ID for tracing
    pub request_id: Arc<str>,
}

impl Request {
    /// Create a new request
    pub fn new(principal: Principal, action: Action, resource: Resource) -> Self {
        Request {
            principal,
            action,
            resource,
            context: Arc::new(BTreeMap::new()),
            request_id: Arc::from(generate_request_id().into_boxed_str()),
        }
    }

    /// Add context to the request
    pub fn with_context(mut self, key: impl Into<String>, value: Value) -> Self {
        let mut ctx = (*self.context).clone();
        ctx.insert(key.into(), value);
        self.context = Arc::new(ctx);
        self
    }

    /// Calculate hash for caching
    pub fn cache_key(&self) -> u64 {
        let mut hasher = AHasher::default();

        // Hash principal
        self.principal.entity.entity_type.hash(&mut hasher);
        self.principal.entity.id.hash(&mut hasher);

        // Hash action
        self.action.name.hash(&mut hasher);
        for (k, v) in self.action.parameters.iter() {
            k.hash(&mut hasher);
            format!("{:?}", v).hash(&mut hasher);
        }

        // Hash resource
        self.resource.entity.entity_type.hash(&mut hasher);
        self.resource.entity.id.hash(&mut hasher);

        // Hash context
        for (k, v) in self.context.iter() {
            k.hash(&mut hasher);
            format!("{:?}", v).hash(&mut hasher);
        }

        hasher.finish()
    }
}

/// Request builder for fluent API
pub struct RequestBuilder {
    principal: Option<Principal>,
    action: Option<Action>,
    resource: Option<Resource>,
    context: BTreeMap<String, Value>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new() -> Self {
        RequestBuilder {
            principal: None,
            action: None,
            resource: None,
            context: BTreeMap::new(),
        }
    }

    /// Set the principal
    pub fn principal(mut self, principal: Principal) -> Self {
        self.principal = Some(principal);
        self
    }

    /// Set the action
    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }

    /// Set the resource
    pub fn resource(mut self, resource: Resource) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Add context
    pub fn context(mut self, key: impl Into<String>, value: Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Build the request
    pub fn build(self) -> crate::Result<Request> {
        let principal = self
            .principal
            .ok_or_else(|| crate::error::RUNEError::InvalidRequest("Missing principal".into()))?;
        let action = self
            .action
            .ok_or_else(|| crate::error::RUNEError::InvalidRequest("Missing action".into()))?;
        let resource = self
            .resource
            .ok_or_else(|| crate::error::RUNEError::InvalidRequest("Missing resource".into()))?;

        let mut request = Request::new(principal, action, resource);
        for (k, v) in self.context {
            request = request.with_context(k, v);
        }

        Ok(request)
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a unique request ID
fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("req_{:x}_{:x}", timestamp, counter)
}
