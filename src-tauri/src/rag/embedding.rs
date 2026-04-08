//! Embedding generation using nomic-embed via llama-server

// Remove unused imports - keep only what's needed
use crate::chat::logging::write_log;
use std::path::Path;

/// Generate embeddings using nomic-embed via llama-server
pub async fn generate_embedding(
    client: &reqwest::Client,
    text: &str,
    server_port: u16,
) -> Result<Vec<f32>, String> {
    let url = format!("http://127.0.0.1:{}/v1/embeddings", server_port);
    
    let request_body = serde_json::json!({
        "input": text,
        "model": "nomic-embed",
    });
    
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to request embedding: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Embedding API error {}: {}", status, body));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse embedding response: {}", e))?;
    
    let embedding = json["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| "No embedding in response".to_string())?
        .iter()
        .filter_map(|v| v.as_f64())
        .map(|v| v as f32)
        .collect::<Vec<f32>>();
    
    Ok(embedding)
}

/// Batch generate embeddings for multiple texts
pub async fn batch_generate_embeddings(
    client: &reqwest::Client,
    texts: &[String],
    server_port: u16,
    log_path: &Path,
) -> Result<Vec<Vec<f32>>, String> {
    write_log(log_path, &format!("Generating {} embeddings", texts.len()));
    
    let mut embeddings = Vec::new();
    
    // Process in batches to avoid overwhelming the server
    let batch_size = 10;
    for chunk in texts.chunks(batch_size) {
        let mut batch_embeddings = Vec::new();
        
        for text in chunk {
            match generate_embedding(client, text, server_port).await {
                Ok(emb) => batch_embeddings.push(emb),
                Err(e) => {
                    write_log(log_path, &format!("Failed to generate embedding: {}", e));
                    return Err(e);
                }
            }
        }
        
        embeddings.extend(batch_embeddings);
        
        // Small delay between batches
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    Ok(embeddings)
}