// File: backend/src/tools/slicap.rs

use super::*;

pub struct SLiCAPAdapter {
    python_path: String,
}

impl SLiCAPAdapter {
    pub fn new() -> Self {
        let python_path = std::env::var("PYTHON_PATH")
            .unwrap_or_else(|_| "python3".to_string());
        
        Self { python_path }
    }
}

#[async_trait]
impl SimulationTool for SLiCAPAdapter {
    fn name(&self) -> &str {
        "slicap"
    }
    
    async fn version(&self) -> Result<String, ToolError> {
        let (stdout, _, _) = run_command_with_timeout(
            &self.python_path,
            &["-c", "import SLiCAP; print(SLiCAP.__version__)"],
            Path::new("."),
            10,
        ).await?;
        
        Ok(stdout.trim().to_string())
    }
    
    async fn is_available(&self) -> bool {
        Command::new(&self.python_path)
            .args(&["-c", "import SLiCAP"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    
    async fn simulate(&self, config: SimulationConfig) -> Result<SimulationResult, ToolError> {
        let start = std::time::Instant::now();
        
        let script = format!(
            r#"
from SLiCAP import *

# Circuit definition
{}

# Run analysis
result = execute()
print("SLiCAP analysis complete")
"#,
            config.netlist
        );
        
        let script_path = write_netlist_file(
            &config.work_dir,
            "slicap_analysis.py",
            &script,
        ).await?;
        
        let (stdout, stderr, exit_code) = run_command_with_timeout(
            &self.python_path,
            &[script_path.to_str().unwrap()],
            &config.work_dir,
            config.timeout_secs,
        ).await?;
        
        let execution_time = start.elapsed();
        
        Ok(SimulationResult {
            success: exit_code == 0,
            stdout,
            stderr,
            output_files: vec![],
            metrics: SimulationMetrics {
                execution_time_ms: execution_time.as_millis() as u64,
                memory_usage_mb: 0,
                exit_code,
            },
        })
    }
}