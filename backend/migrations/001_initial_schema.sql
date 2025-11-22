-- File: backend/migrations/001_initial_schema.sql

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_login TIMESTAMPTZ
);

-- Projects table
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    owner UUID REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- IDEF0 Blocks (nested hierarchy)
CREATE TABLE idef_blocks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
    parent_block_id UUID REFERENCES idef_blocks(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    block_type VARCHAR(50) NOT NULL, -- 'source', 'amplifier', 'adc', etc.
    
    -- Signal specifications (stored as JSONB for flexibility)
    signal_specs JSONB,
    -- Example: {"bandwidth": "1MHz", "amplitude": "5V", "type": "analog"}
    
    -- Position in flow diagram
    position_x FLOAT,
    position_y FLOAT,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Circuit views (schematic inside IDEF block)
CREATE TABLE circuit_views (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idef_block_id UUID REFERENCES idef_blocks(id) ON DELETE CASCADE,
    
    -- Circuit data (components and connections as JSON)
    schematic_data JSONB,
    -- Example: {"components": [...], "nets": [...]}
    
    -- Generated netlist (cached)
    spice_netlist TEXT,
    netlist_hash VARCHAR(64), -- SHA256 of schematic_data
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Components (for granular locking)
CREATE TABLE components (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circuit_view_id UUID REFERENCES circuit_views(id) ON DELETE CASCADE,
    component_type VARCHAR(50) NOT NULL, -- 'resistor', 'capacitor', etc.
    
    -- Component properties
    properties JSONB,
    -- Example: {"value": "10k", "tolerance": "5%"}
    
    -- Visual position
    position_x FLOAT,
    position_y FLOAT,
    
    -- Lock management
    locked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    lock_expires TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Connections/Nets
CREATE TABLE nets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circuit_view_id UUID REFERENCES circuit_views(id) ON DELETE CASCADE,
    name VARCHAR(255),
    
    -- Connected pins (array of component_id:pin_number)
    connected_pins JSONB,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Simulation results metadata (actual data in MinIO/filesystem)
CREATE TABLE simulation_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circuit_view_id UUID REFERENCES circuit_views(id) ON DELETE CASCADE,
    
    tool_used VARCHAR(50), -- 'ngspice', 'verilator', etc.
    status VARCHAR(20), -- 'running', 'completed', 'failed'
    
    -- Reference to stored results
    results_path TEXT, -- MinIO path or filesystem path
    
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Indexes for performance
CREATE INDEX idx_idef_blocks_project ON idef_blocks(project_id);
CREATE INDEX idx_idef_blocks_parent ON idef_blocks(parent_block_id);
CREATE INDEX idx_circuit_views_block ON circuit_views(idef_block_id);
CREATE INDEX idx_components_circuit ON components(circuit_view_id);
CREATE INDEX idx_components_locked ON components(locked_by, lock_expires);
CREATE INDEX idx_nets_circuit ON nets(circuit_view_id);

-- Function to auto-update updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers
CREATE TRIGGER update_projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_circuit_views_updated_at
    BEFORE UPDATE ON circuit_views
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();