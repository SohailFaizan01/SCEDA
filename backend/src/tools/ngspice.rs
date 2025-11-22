// File: backend/src/tools/ngspice.rs

use super::*;

pub struct NgspiceAdapter {
    binary_path: String,
}

impl NgspiceAdapter {
    pub fn new() -> Self {
        // Try to find ngspice in PATH or common locations
        let binary_path = std::env::var("NGSPICE_PATH")
            .unwrap_or_else(|_| "ngspice".to_string());
        
        Self { binary_path }
    }
    
    async fn parse_raw_file(&self, path: &Path) -> Result<Vec<u8>, ToolError> {
        // Read the .raw output file
        tokio::fs::read(path).await.map_err(|e| e.into())
    }
}

#[async_trait]
impl SimulationTool for NgspiceAdapter {
    fn name(&self) -> &str {
        "ngspice"
    }
    
    async fn version(&self) -> Result<String, ToolError> {
        let (stdout, _, _) = run_command_with_timeout(
            &self.binary_path,
            &["--version"],
            Path::new("."),
            10,
        ).await?;
        
        // Parse version from output
        let version = stdout.lines()
            .next()
            .unwrap_or("unknown")
            .to_string();
        
        Ok(version)
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
        
        // Write netlist to file
        let netlist_path = write_netlist_file(
            &config.work_dir,
            "circuit.cir",
            &config.netlist,
        ).await?;
        
        // Output file path
        let output_path = config.work_dir.join("output.raw");
        
        // Run ngspice in batch mode
        let args = vec![
            "-b",  // Batch mode
            netlist_path.to_str().unwrap(),
            "-r",  // Raw output file
            output_path.to_str().unwrap(),
        ];
        
        log::debug!("Running: {} {:?}", self.binary_path, args);
        
        let (stdout, stderr, exit_code) = run_command_with_timeout(
            &self.binary_path,
            &args.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
            &config.work_dir,
            config.timeout_secs,
        ).await?;
        
        let execution_time = start.elapsed();
        
        // Check if simulation succeeded
        let success = exit_code == 0 && output_path.exists();
        
        let mut output_files = vec![];
        if output_path.exists() {
            output_files.push(output_path.clone());
        }
        
        // Also look for .log files
        if let Ok(mut entries) = tokio::fs::read_dir(&config.work_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "log" {
                        output_files.push(path);
                    }
                }
            }
        }
        
        Ok(SimulationResult {
            success,
            stdout,
            stderr,
            output_files,
            metrics: SimulationMetrics {
                execution_time_ms: execution_time.as_millis() as u64,
                memory_usage_mb: 0, // TODO: Get from system
                exit_code,
            },
        })
    }
}

// ============ RAW FILE PARSER (Optional - for waveform data) ============

#[derive(Debug)]
pub struct SpiceWaveform {
    pub variables: Vec<String>,
    pub data: Vec<Vec<f64>>,
}

pub fn parse_ngspice_raw(data: &[u8]) -> Result<SpiceWaveform, ToolError> {
    // Simplified parser - in production, use a proper library
    // or implement full binary/ASCII raw file parsing
    
    // For now, just return empty waveform
    Ok(SpiceWaveform {
        variables: vec!["time".to_string()],
        data: vec![],
    })
}