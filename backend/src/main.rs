use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use sqlx::postgres::PgPoolOptions;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Mutex;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

mod storage;
mod tools;
mod encryption;
mod models;

use models::{Project, Schematic, Flow};  // Import from single file

struct AppState {
    db_pool: sqlx::PgPool,
    projects: Mutex<HashMap<String, Project>>,
    storage: Box<dyn storage::Storage>,
    tool_manager: tools::ToolManager,
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    owner: String,
}

#[derive(Deserialize)]
struct LockComponentRequest {
    project_id: String,
    component_id: String,
    user: String,
}

#[derive(Deserialize)]
struct UploadFileRequest {
    project_id: String,
    filename: String,
    data: String,
}

#[derive(Deserialize)]
struct SimulateRequest {
    tool: String,
    project_id: String,
    netlist: String,
    parameters: Option<Vec<(String, String)>>,
}
// Convert struct to JSON
#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
    data: Option<serde_json::Value>,
}
// Return server running response to client
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Server is running".to_string(),
        data: None,
    })
}

async fn create_project(
    data: web::Data<AppState>,
    req: web::Json<CreateProjectRequest>,
) -> impl Responder {
    let project_id = Uuid::new_v4().to_string();
    
    let project = Project {
        id: project_id.clone(),
        name: req.name.clone(),
        owner: req.owner.clone(),
        schematics: Vec::new(),
        flows: Vec::new(),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };
    
    let result = sqlx::query(
        "INSERT INTO projects (id, name, owner) VALUES ($1, $2, $3)"
    )
    .bind(&project.id)
    .bind(&project.name)
    .bind(&project.owner)
    .execute(&data.db_pool)
    .await;
    
    match result {
        Ok(_) => {
            let mut projects = data.projects.lock().unwrap();
            projects.insert(project_id.clone(), project.clone());
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "Project created".to_string(),
                data: Some(serde_json::to_value(project).unwrap()),
            })
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            })
        }
    }
}

async fn lock_component(
    data: web::Data<AppState>,
    req: web::Json<LockComponentRequest>,
) -> impl Responder {
    let mut projects = data.projects.lock().unwrap();
    
    if let Some(project) = projects.get_mut(&req.project_id) {
        if let Some(component) = project.schematics.iter_mut()
            .find(|c| c.id.to_string() == req.component_id) {
            
            let now = Utc::now();
            
            if let Some(expires) = component.lock_expires {
                if expires > now && component.locked_by.is_some() {
                    return HttpResponse::Conflict().json(ApiResponse {
                        success: false,
                        message: format!("Component locked by {}", 
                                       component.locked_by.as_ref().unwrap()),
                        data: None,
                    });
                }
            }
            
            component.locked_by = Some(req.user.clone());
            component.lock_expires = Some(now + Duration::minutes(2));
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "Component locked".to_string(),
                data: Some(serde_json::json!({
                    "expires_at": component.lock_expires
                })),
            })
        } else {
            HttpResponse::NotFound().json(ApiResponse {
                success: false,
                message: "Component not found".to_string(),
                data: None,
            })
        }
    } else {
        HttpResponse::NotFound().json(ApiResponse {
            success: false,
            message: "Project not found".to_string(),
            data: None,
        })
    }
}

async fn get_project(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let project_id = path.into_inner();
    let projects = data.projects.lock().unwrap();
    
    if let Some(project) = projects.get(&project_id) {
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "Project found".to_string(),
            data: Some(serde_json::to_value(project).unwrap()),
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse {
            success: false,
            message: "Project not found".to_string(),
            data: None,
        })
    }
}

async fn upload_file(
    state: web::Data<AppState>,
    req: web::Json<UploadFileRequest>,
) -> impl Responder {
    use base64::{Engine as _, engine::general_purpose};
    
    let data = match general_purpose::STANDARD.decode(&req.data) {
        Ok(d) => d,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse {
                success: false,
                message: format!("Invalid base64: {}", e),
                data: None,
            });
        }
    };
    
    match state.storage.save_file(&req.project_id, &req.filename, &data).await {
        Ok(path) => {
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "File uploaded".to_string(),
                data: Some(serde_json::json!({
                    "path": path,
                    "size": data.len()
                })),
            })
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: format!("Upload failed: {}", e),
                data: None,
            })
        }
    }
}

async fn download_file(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let file_path = path.into_inner();
    
    match state.storage.load_file(&file_path).await {
        Ok(data) => {
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(data)
        }
        Err(e) => {
            HttpResponse::NotFound().json(ApiResponse {
                success: false,
                message: format!("File not found: {}", e),
                data: None,
            })
        }
    }
}

async fn list_tools(state: web::Data<AppState>) -> impl Responder {
    let tools = state.tool_manager.list_tools();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("{} tools available", tools.len()),
        data: Some(serde_json::json!({"tools": tools})),
    })
}

async fn run_simulation(
    state: web::Data<AppState>,
    req: web::Json<SimulateRequest>,
) -> impl Responder {
    let params = req.parameters.clone().unwrap_or_default();
    
    match state.tool_manager.simulate(
        &req.tool,
        &req.netlist,
        &req.project_id,
        params,
    ).await {
        Ok(result) => {
            let result_id = Uuid::new_v4().to_string();
            
            if !result.stdout.is_empty() {
                let _ = state.storage.save_file(
                    &req.project_id,
                    &format!("sim_{}_stdout.log", result_id),
                    result.stdout.as_bytes(),
                ).await;
            }
            
            for output_file in &result.output_files {
                if let Ok(data) = tokio::fs::read(output_file).await {
                    let filename = output_file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("output.dat");
                    
                    let _ = state.storage.save_file(
                        &req.project_id,
                        &format!("sim_{}_{}", result_id, filename),
                        &data,
                    ).await;
                }
            }
            
            HttpResponse::Ok().json(ApiResponse {
                success: result.success,
                message: if result.success { "Simulation completed" } else { "Simulation failed" }.to_string(),
                data: Some(serde_json::json!({
                    "result_id": result_id,
                    "metrics": result.metrics,
                    "output_files": result.output_files.len(),
                })),
            })
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: format!("Simulation error: {}", e),
                data: None,
            })
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://eda_user:eda_pass@localhost/eda_platform".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");
    
    let storage = storage::create_storage()
        .await
        .expect("Failed to initialize MinIO storage");
    
    let work_dir = std::env::var("TOOL_WORK_DIR")
        .unwrap_or_else(|_| "/tmp/eda-sims".to_string());
    let tool_manager = tools::ToolManager::new(std::path::PathBuf::from(work_dir));
    
    let available_tools = tool_manager.check_all_tools().await;
    println!("📊 Tool availability:");
    for (name, available) in available_tools {
        println!("   {} {}", if available { "✓" } else { "✗" }, name);
    }
    
    println!("🚀 Server starting on http://0.0.0.0:8080");
    println!("💾 Storage backend: MinIO");
    
    let app_state = web::Data::new(AppState {
        db_pool: pool,
        projects: Mutex::new(HashMap::new()),
        storage,
        tool_manager,
    });
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .route("/health", web::get().to(health_check))
            .route("/api/projects", web::post().to(create_project))
            .route("/api/projects/{id}", web::get().to(get_project))
            .route("/api/lock", web::post().to(lock_component))
            .route("/api/files/upload", web::post().to(upload_file))
            .route("/api/files/{path:.*}", web::get().to(download_file))
            .route("/api/tools/list", web::get().to(list_tools))
            .route("/api/simulate", web::post().to(run_simulation))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}