// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod file_manager;
mod ai_client;

use tauri::{Manager, State};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::ai_client::AIClient;
use crate::file_manager::FileManager;

#[derive(Debug, Clone, serde::Serialize)]
struct AppState {
    config: Arc<RwLock<Config>>,
    ai_client: Arc<AIClient>,
    file_manager: Arc<RwLock<FileManager>>,
}

#[tauri::command]
async fn get_health(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.ai_client.health().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn complete_code(
    prompt: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let request = ai_client::CompletionRequest {
        prompt,
        language,
        model,
        provider,
        max_tokens: Some(1000),
        temperature: Some(0.7),
        system_prompt: None,
    };
    
    state.ai_client.complete(request).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn analyze_code(
    code: String,
    language: String,
    analysis_type: String,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let request = ai_client::AnalysisRequest {
        code,
        language,
        analysis_type,
        context: None,
    };
    
    state.ai_client.analyze(request).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn code_action(
    code: String,
    language: String,
    action: String,
    instructions: Option<String>,
    target_language: Option<String>,
    state: State<'_, AppState>
) -> Result<serde_json::Value, String> {
    let request = ai_client::CodeActionRequest {
        code,
        language,
        action,
        instructions,
        target_language,
    };
    
    state.ai_client.code_action(request).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_file(
    path: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let mut file_manager = state.file_manager.write().await;
    file_manager.open_file(&path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_file(
    path: String,
    content: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut file_manager = state.file_manager.write().await;
    file_manager.save_file(&path, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_files(
    directory: String,
    state: State<'_, AppState>
) -> Result<Vec<String>, String> {
    let file_manager = state.file_manager.read().await;
    file_manager.list_files(&directory)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.read().await;
    serde_json::to_value(&*config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_config(
    new_config: serde_json::Value,
    state: State<'_, AppState>
) -> Result<(), String> {
    let mut config = state.config.write().await;
    *config = serde_json::from_value(new_config)
        .map_err(|e| e.to_string())?;
    
    config.save()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_providers(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.ai_client.providers().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_models(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.ai_client.models().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_metrics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state.ai_client.metrics().await
        .map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() {
    // Initialize configuration
    let config = Arc::new(RwLock::new(
        Config::load().unwrap_or_else(|_| Config::default())
    ));

    // Initialize AI client
    let server_url = {
        let config_guard = config.read().await;
        config_guard.server.url.clone()
    };
    let ai_client = Arc::new(AIClient::new(&server_url));

    // Initialize file manager
    let file_manager = Arc::new(RwLock::new(FileManager::new()));

    let app_state = AppState {
        config,
        ai_client,
        file_manager,
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_health,
            complete_code,
            analyze_code,
            code_action,
            open_file,
            save_file,
            list_files,
            get_config,
            update_config,
            get_providers,
            get_models,
            get_metrics
        ])
        .setup(|app| {
            // Set window title
            let window = app.get_window("main").unwrap();
            window.set_title("UAIDA - Universal AI Development Assistant").unwrap();
            
            // Set window icon
            #[cfg(desktop)]
            {
                use tauri::Icon;
                if let Ok(icon) = Icon::from_path("icons/icon.png") {
                    window.set_icon(icon).ok();
                }
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}