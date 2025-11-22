// File: backend/src/storage/mod.rs

use async_trait::async_trait;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;
use serde::{Deserialize, Serialize};

// ============ STORAGE TRAIT ============

#[async_trait]
pub trait Storage: Send + Sync {
    async fn save_file(&self, project_id: &str, filename: &str, data: &[u8]) 
        -> Result<String, StorageError>;
    
    async fn load_file(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    
    async fn delete_file(&self, path: &str) -> Result<(), StorageError>;
    
    async fn list_files(&self, project_id: &str) -> Result<Vec<FileInfo>, StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub enum StorageError {
    NotFound(String),
    InvalidPath(String),
    UploadFailed(String),
    ConnectionError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "File not found: {}", p),
            Self::InvalidPath(p) => write!(f, "Invalid path: {}", p),
            Self::UploadFailed(e) => write!(f, "Upload failed: {}", e),
            Self::ConnectionError(e) => write!(f, "Connection error: {}", e),
        }
    }
}

impl std::error::Error for StorageError {}

// ============ MINIO STORAGE ============

pub struct MinioStorage {
    bucket: Bucket,
}

impl MinioStorage {
    pub async fn new(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket_name: &str,
        region: &str,
    ) -> Result<Self, StorageError> {
        
        let credentials = Credentials::new(
            Some(access_key),
            Some(secret_key),
            None,
            None,
            None,
        ).map_err(|e| StorageError::ConnectionError(format!("Credentials error: {}", e)))?;
        
        let region = Region::Custom {
            region: region.to_string(),
            endpoint: endpoint.to_string(),
        };
        
        let bucket = Bucket::new(bucket_name, region, credentials)
            .map_err(|e| StorageError::ConnectionError(format!("Bucket error: {}", e)))?
            .with_path_style();
        
        // Create bucket if it doesn't exist
        match bucket.create().await {
            Ok(_) => log::info!("Created MinIO bucket: {}", bucket_name),
            Err(_) => log::debug!("MinIO bucket already exists: {}", bucket_name),
        }
        
        log::info!("✓ MinIO storage initialized: {}/{}", endpoint, bucket_name);
        Ok(Self { bucket })
    }
    
    pub fn from_env() -> Result<Self, StorageError> {
        let endpoint = std::env::var("MINIO_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:9000".to_string());
        let access_key = std::env::var("MINIO_ACCESS_KEY")
            .unwrap_or_else(|_| "minioadmin".to_string());
        let secret_key = std::env::var("MINIO_SECRET_KEY")
            .unwrap_or_else(|_| "minioadmin".to_string());
        let bucket = std::env::var("MINIO_BUCKET")
            .unwrap_or_else(|_| "eda-platform".to_string());
        let region = std::env::var("MINIO_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string());
        
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::new(&endpoint, &access_key, &secret_key, &bucket, &region))
    }
    
    fn object_key(&self, project_id: &str, filename: &str) -> String {
        format!("projects/{}/{}", project_id, filename)
    }
    
    fn parse_path(&self, path: &str) -> Result<String, StorageError> {
        let key = path.strip_prefix("s3://")
            .ok_or_else(|| StorageError::InvalidPath(path.to_string()))?;
        
        let key = key.strip_prefix(&format!("{}/", self.bucket.name))
            .unwrap_or(key);
        
        Ok(key.to_string())
    }
}

#[async_trait]
impl Storage for MinioStorage {
    async fn save_file(&self, project_id: &str, filename: &str, data: &[u8]) 
        -> Result<String, StorageError> {
        
        let key = self.object_key(project_id, filename);
        
        self.bucket.put_object(&key, data).await
            .map_err(|e| StorageError::UploadFailed(format!("{:?}", e)))?;
        
        log::info!("✓ Saved to MinIO: {} ({} bytes)", key, data.len());
        Ok(format!("s3://{}/{}", self.bucket.name, key))
    }
    
    async fn load_file(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let key = self.parse_path(path)?;
        
        let response = self.bucket.get_object(&key).await
            .map_err(|e| StorageError::NotFound(format!("{}: {:?}", path, e)))?;
        
        Ok(response.bytes().to_vec())
    }
    
    async fn delete_file(&self, path: &str) -> Result<(), StorageError> {
        let key = self.parse_path(path)?;
        
        self.bucket.delete_object(&key).await
            .map_err(|e| StorageError::UploadFailed(format!("{:?}", e)))?;
        
        Ok(())
    }
    
    async fn list_files(&self, project_id: &str) -> Result<Vec<FileInfo>, StorageError> {
        let prefix = format!("projects/{}/", project_id);
        
        let results = self.bucket.list(prefix.clone(), None).await
            .map_err(|e| StorageError::UploadFailed(format!("{:?}", e)))?;
        
        let mut files = Vec::new();
        
        for list in results {
            for object in list.contents {
                let filename = object.key.strip_prefix(&prefix)
                    .unwrap_or(&object.key)
                    .to_string();
                
                files.push(FileInfo {
                    name: filename,
                    path: format!("s3://{}/{}", self.bucket.name, object.key),
                    size_bytes: object.size,
                    created_at: chrono::DateTime::parse_from_rfc3339(&object.last_modified)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(chrono::Utc::now),
                });
            }
        }
        
        Ok(files)
    }
}

// ============ FACTORY FUNCTION ============

pub async fn create_storage() -> Result<Box<dyn Storage>, StorageError> {
    let storage = MinioStorage::from_env()?;
    Ok(Box::new(storage))
}