// File: backend/src/tools/mod.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub mod ngspice;
pub mod verilator;
pub mod slicap;
pub mod openlane;

// ============ TOOL TRAIT ============

#[async_trait]
pub trait SimulationTool: Send + Sync {
    /// Tool name (e.g., "ngspice", "verilator")
    fn name(&self) -> &str;
    
    /// Tool version check
    async fn version(&self) -> Result<String, ToolError>;
    
    /// Check if tool is installed and accessible
    async fn is_available(&self) -> bool;
    
    /// Run simulation with netlist/HDL and return results
    async fn simulate(&self, config: SimulationConfig) -> Result<SimulationResult, ToolError>;
}

// ============ COMMON TYPES ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub project_id: String,
    pub netlist: String,           // SPICE netlist or Verilog code
    pub work_dir: PathBuf,         // Temporary working directory
    pub timeout_secs: u64,         // Max execution time
    pub parameters: Vec<(String, String)>, // Tool-specific params
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub output_files: Vec<PathBuf>, // Generated files (.raw, .vcd, etc.)
    pub metrics: SimulationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationMetrics {
    pub execution_time_ms: u64,
    pub memory_usage_mb: u64,
    pub exit_code: i32,
}

#[derive(Debug)]
pub enum ToolError {
    NotInstalled(String),
    ExecutionFailed(String),
    Timeout,
    ParseError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstalled(t) => write!(f, "Tool not installed: {}", t),
            Self::ExecutionFailed(e) => write!(f, "Execution failed: {}", e),
            Self::Timeout => write!(f, "Simulation timed out"),
            Self::ParseError(e) => write!(f, "Parse error: {}", e),
            Self::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ToolError {}

impl From<std::io::Error> for ToolError {
    fn from(e: std::io::Error) -> Self {
        ToolError::IoError(e)
    }
}

// ============ TOOL MANAGER ============

pub struct ToolManager {
    tools: std::collections::HashMap<String, Box<dyn SimulationTool>>,
    work_dir_base: PathBuf,
}

impl ToolManager {
    pub fn new(work_dir: PathBuf) -> Self {
        let mut manager = Self {
            tools: std::collections::HashMap::new(),
            work_dir_base: work_dir,
        };
        
        // Register all tools
        manager.register_tool(Box::new(ngspice::NgspiceAdapter::new()));
        manager.register_tool(Box::new(verilator::VerilatorAdapter::new()));
        manager.register_tool(Box::new(slicap::SLiCAPAdapter::new()));
        manager.register_tool(Box::new(openlane::OpenLaneAdapter::new()));
        
        manager
    }
    
    pub fn register_tool(&mut self, tool: Box<dyn SimulationTool>) {
        let name = tool.name().to_string();
        log::info!("Registered tool: {}", name);
        self.tools.insert(name, tool);
    }
    
    pub async fn check_all_tools(&self) -> Vec<(String, bool)> {
        let mut results = Vec::new();
        
        for (name, tool) in &self.tools {
            let available = tool.is_available().await;
            results.push((name.clone(), available));
            
            if available {
                if let Ok(version) = tool.version().await {
                    log::info!("✓ {} available: {}", name, version);
                } else {
                    log::warn!("✓ {} available but version unknown", name);
                }
            } else {
                log::warn!("✗ {} not found", name);
            }
        }
        
        results
    }
    
    pub async fn simulate(
        &self,
        tool_name: &str,
        netlist: &str,
        project_id: &str,
        params: Vec<(String, String)>,
    ) -> Result<SimulationResult, ToolError> {
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| ToolError::NotInstalled(tool_name.to_string()))?;
        
        // Create unique work directory
        let run_id = Uuid::new_v4();
        let work_dir = self.work_dir_base
            .join(project_id)
            .join(run_id.to_string());
        
        tokio::fs::create_dir_all(&work_dir).await?;
        
        let config = SimulationConfig {
            project_id: project_id.to_string(),
            netlist: netlist.to_string(),
            work_dir,
            timeout_secs: 300, // 5 minutes default
            parameters: params,
        };
        
        log::info!("Starting {} simulation for project {}", tool_name, project_id);
        let start = std::time::Instant::now();
        
        let result = tool.simulate(config).await?;
        
        log::info!("✓ {} simulation completed in {}ms", 
                   tool_name, result.metrics.execution_time_ms);
        
        Ok(result)
    }
    
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
}

// ============ HELPER FUNCTIONS ============

pub async fn run_command_with_timeout(
    command: &str,
    args: &[&str],
    work_dir: &Path,
    timeout_secs: u64,
) -> Result<(String, String, i32), ToolError> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    let start = std::time::Instant::now();
    
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        cmd.output()
    )
    .await
    .map_err(|_| ToolError::Timeout)?
    .map_err(|e| ToolError::ExecutionFailed(format!("{}", e)))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    
    Ok((stdout, stderr, exit_code))
}

pub async fn write_netlist_file(
    work_dir: &Path,
    filename: &str,
    content: &str,
) -> Result<PathBuf, ToolError> {
    let path = work_dir.join(filename);
    
    let mut file = tokio::fs::File::create(&path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    
    Ok(path)
}