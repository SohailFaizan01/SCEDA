// File: backend/src/tools/verilator.rs

use super::*;

pub struct VerilatorAdapter {
    binary_path: String,
}

impl VerilatorAdapter {
    pub fn new() -> Self {
        let binary_path = std::env::var("VERILATOR_PATH")
            .unwrap_or_else(|_| "verilator".to_string());
        
        Self { binary_path }
    }
}

#[async_trait]
impl SimulationTool for VerilatorAdapter {
    fn name(&self) -> &str {
        "verilator"
    }
    
    async fn version(&self) -> Result<String, ToolError> {
        let (stdout, _, _) = run_command_with_timeout(
            &self.binary_path,
            &["--version"],
            Path::new("."),
            10,
        ).await?;
        
        Ok(stdout.lines().next().unwrap_or("unknown").to_string())
    }
    
    async fn is_available(&self) -> bool {
        Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .await
            .is_ok()
    }
    
    async fn simulate(&self, config: SimulationConfig) -> Result<SimulationResult, ToolError> {
        let start = std::time::Instant::now();
        
        // Write Verilog to file
        let verilog_path = write_netlist_file(
            &config.work_dir,
            "design.v",
            &config.netlist,
        ).await?;
        
        // Step 1: Lint check
        log::debug!("Running Verilator lint check...");
        let lint_args = vec!["--lint-only", "-Wall", verilog_path.to_str().unwrap()];
        
        let (lint_stdout, lint_stderr, lint_exit) = run_command_with_timeout(
            &self.binary_path,
            &lint_args.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
            &config.work_dir,
            config.timeout_secs,
        ).await?;
        
        if lint_exit != 0 {
            return Ok(SimulationResult {
                success: false,
                stdout: lint_stdout,
                stderr: format!("Lint failed:\n{}", lint_stderr),
                output_files: vec![],
                metrics: SimulationMetrics {
                    execution_time_ms: start.elapsed().as_millis() as u64,
                    memory_usage_mb: 0,
                    exit_code: lint_exit,
                },
            });
        }
        
        // Step 2: Compile (if lint passed)
        log::debug!("Compiling with Verilator...");
        let compile_args = vec![
            "-Wall",
            "--cc",  // Generate C++ code
            "--exe", // Make executable
            "--build",
            verilog_path.to_str().unwrap(),
        ];
        
        let (compile_stdout, compile_stderr, compile_exit) = run_command_with_timeout(
            &self.binary_path,
            &compile_args.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
            &config.work_dir,
            config.timeout_secs,
        ).await?;
        
        let execution_time = start.elapsed();
        
        // Find generated files
        let mut output_files = vec![];
        let obj_dir = config.work_dir.join("obj_dir");
        if obj_dir.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&obj_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    output_files.push(entry.path());
                }
            }
        }
        
        Ok(SimulationResult {
            success: compile_exit == 0,
            stdout: format!("{}\n{}", lint_stdout, compile_stdout),
            stderr: format!("{}\n{}", lint_stderr, compile_stderr),
            output_files,
            metrics: SimulationMetrics {
                execution_time_ms: execution_time.as_millis() as u64,
                memory_usage_mb: 0,
                exit_code: compile_exit,
            },
        })
    }
}