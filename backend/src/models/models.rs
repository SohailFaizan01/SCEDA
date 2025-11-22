// File: backend/src/models.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============ PROJECT ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub schematics: Vec<Schematic>,
    pub flows: Vec<Flow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

// ============ COMPONENT TYPES ============

// ============ IDEF0 BLOCK ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IDEF0Block {
    pub id: Uuid,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_block_id: Option<Uuid>,
    pub name: String,
    pub block_type: String,
    pub signal_specs: serde_json::Value,
    pub position_x: f64,
    pub position_y: f64,
}

// ============ CIRCUIT VIEW ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitView {
    pub id: Uuid,
    pub idef_block_id: Uuid,
    pub schematic_data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spice_netlist: Option<String>,
}

// ============ COMPONENT ============

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schematic {
    pub id: Uuid,
    pub circuit_view_id: Option<Uuid>,
    pub parent_id: String,
    pub component_type: String,
    pub properties: serde_json::Value,
    pub position_x: f64,
    pub position_y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_expires: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Flow {
    pub id: Uuid,
    pub parent_id: String,
    pub name: String,
    pub component_type: String, // e.g., "Process", "Decision"
    pub position_x: f64,
    pub position_y: f64,
}

// ============ NET ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Net {
    pub id: Uuid,
    pub circuit_view_id: Uuid,
    pub name: String,
    pub connected_pins: serde_json::Value,  // Array of "component_id:pin_number"
}