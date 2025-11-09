//! Type system for RUNE

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Core value type in RUNE
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// String value
    String(Arc<str>),
    /// Array of values
    Array(Arc<[Value]>),
    /// Object/map of values
    Object(Arc<BTreeMap<String, Value>>),
}

impl Value {
    /// Create a string value
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(Arc::from(s.into().into_boxed_str()))
    }

    /// Create an array value
    pub fn array(values: Vec<Value>) -> Self {
        Value::Array(Arc::from(values.into_boxed_slice()))
    }

    /// Create an object value
    pub fn object(map: BTreeMap<String, Value>) -> Self {
        Value::Object(Arc::new(map))
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Integer(i) => *i != 0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
        }
    }
}

/// Entity in the RUNE system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity {
    /// Entity type (e.g., "User", "Agent", "Resource")
    pub entity_type: Arc<str>,
    /// Entity ID
    pub id: Arc<str>,
    /// Entity attributes
    pub attributes: Arc<BTreeMap<String, Value>>,
    /// Parent entities (for hierarchies)
    pub parents: Vec<Entity>,
}

impl Entity {
    /// Create a new entity
    pub fn new(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Entity {
            entity_type: Arc::from(entity_type.into().into_boxed_str()),
            id: Arc::from(id.into().into_boxed_str()),
            attributes: Arc::new(BTreeMap::new()),
            parents: Vec::new(),
        }
    }

    /// Add an attribute to the entity
    pub fn with_attribute(mut self, key: impl Into<String>, value: Value) -> Self {
        let mut attrs = (*self.attributes).clone();
        attrs.insert(key.into(), value);
        self.attributes = Arc::new(attrs);
        self
    }

    /// Add a parent entity
    pub fn with_parent(mut self, parent: Entity) -> Self {
        self.parents.push(parent);
        self
    }
}

/// Principal making the request
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Principal {
    /// The entity representing the principal
    pub entity: Entity,
}

impl Principal {
    /// Create a new principal
    pub fn new(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Principal {
            entity: Entity::new(entity_type, id),
        }
    }

    /// Create an agent principal
    pub fn agent(id: impl Into<String>) -> Self {
        Self::new("Agent", id)
    }

    /// Create a user principal
    pub fn user(id: impl Into<String>) -> Self {
        Self::new("User", id)
    }
}

/// Action being performed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Action {
    /// Action name
    pub name: Arc<str>,
    /// Action parameters
    pub parameters: Arc<BTreeMap<String, Value>>,
}

impl Action {
    /// Create a new action
    pub fn new(name: impl Into<String>) -> Self {
        Action {
            name: Arc::from(name.into().into_boxed_str()),
            parameters: Arc::new(BTreeMap::new()),
        }
    }

    /// Add a parameter to the action
    pub fn with_parameter(mut self, key: impl Into<String>, value: Value) -> Self {
        let mut params = (*self.parameters).clone();
        params.insert(key.into(), value);
        self.parameters = Arc::new(params);
        self
    }
}

/// Resource being accessed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    /// The entity representing the resource
    pub entity: Entity,
}

impl Resource {
    /// Create a new resource
    pub fn new(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Resource {
            entity: Entity::new(entity_type, id),
        }
    }

    /// Create a file resource
    pub fn file(path: impl Into<String>) -> Self {
        Self::new("File", path)
    }

    /// Create a database resource
    pub fn database(name: impl Into<String>) -> Self {
        Self::new("Database", name)
    }

    /// Create an API resource
    pub fn api(endpoint: impl Into<String>) -> Self {
        Self::new("API", endpoint)
    }
}
