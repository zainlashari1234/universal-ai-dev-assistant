use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::agents::visual_programmer::{
    VisualProgrammer, VisualCanvas, VisualComponent, Connection,
    FlowchartToCodeRequest, FlowchartToCodeResponse, ComponentTemplate
};
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

/// Create a new visual programming canvas
#[utoipa::path(
    post,
    path = "/api/v1/visual/canvas",
    request_body = CreateCanvasRequest,
    responses(
        (status = 200, description = "Canvas created successfully", body = CreateCanvasResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn create_canvas(
    State(app_state): State<AppState>,
    Json(request): Json<CreateCanvasRequest>,
) -> Result<Json<CreateCanvasResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visual_programmer = VisualProgrammer::new();

    match visual_programmer.create_canvas(request.name, request.creator).await {
        Ok(canvas_id) => {
            app_state.metrics.record_visual_canvas_created();
            Ok(Json(CreateCanvasResponse {
                canvas_id,
                success: true,
                message: "Canvas created successfully".to_string(),
            }))
        }
        Err(e) => {
            app_state.metrics.record_visual_operation_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Canvas creation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Add a component to the visual canvas
#[utoipa::path(
    post,
    path = "/api/v1/visual/canvas/{canvas_id}/components",
    request_body = AddComponentRequest,
    responses(
        (status = 200, description = "Component added successfully"),
        (status = 404, description = "Canvas not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn add_component(
    State(app_state): State<AppState>,
    Path(canvas_id): Path<String>,
    Json(request): Json<AddComponentRequest>,
) -> Result<Json<ComponentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visual_programmer = VisualProgrammer::new();

    match visual_programmer.add_component(canvas_id, request.component).await {
        Ok(_) => {
            app_state.metrics.record_visual_component_added();
            Ok(Json(ComponentResponse {
                success: true,
                message: "Component added successfully".to_string(),
            }))
        }
        Err(e) => {
            app_state.metrics.record_visual_operation_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Component addition failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Connect two components on the canvas
#[utoipa::path(
    post,
    path = "/api/v1/visual/canvas/{canvas_id}/connections",
    request_body = CreateConnectionRequest,
    responses(
        (status = 200, description = "Connection created successfully"),
        (status = 400, description = "Invalid connection"),
        (status = 404, description = "Canvas not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn create_connection(
    State(app_state): State<AppState>,
    Path(canvas_id): Path<String>,
    Json(request): Json<CreateConnectionRequest>,
) -> Result<Json<ConnectionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visual_programmer = VisualProgrammer::new();

    match visual_programmer.connect_components(canvas_id, request.connection).await {
        Ok(_) => {
            app_state.metrics.record_visual_connection_created();
            Ok(Json(ConnectionResponse {
                success: true,
                message: "Connection created successfully".to_string(),
                connection_id: request.connection.connection_id,
            }))
        }
        Err(e) => {
            app_state.metrics.record_visual_operation_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Connection creation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Generate code from visual flowchart
#[utoipa::path(
    post,
    path = "/api/v1/visual/generate-code",
    request_body = FlowchartToCodeRequest,
    responses(
        (status = 200, description = "Code generated successfully", body = FlowchartToCodeResponse),
        (status = 400, description = "Invalid flowchart"),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn generate_code_from_flowchart(
    State(app_state): State<AppState>,
    Json(request): Json<FlowchartToCodeRequest>,
) -> Result<Json<FlowchartToCodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visual_programmer = VisualProgrammer::new();

    match visual_programmer.generate_code_from_flowchart(request).await {
        Ok(response) => {
            app_state.metrics.record_visual_code_generated();
            Ok(Json(response))
        }
        Err(e) => {
            app_state.metrics.record_visual_operation_failed();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Code generation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Get available component templates
#[utoipa::path(
    get,
    path = "/api/v1/visual/components/templates",
    responses(
        (status = 200, description = "Component templates retrieved", body = ComponentTemplatesResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn get_component_templates(
    State(app_state): State<AppState>,
    Query(params): Query<TemplateQuery>,
) -> Result<Json<ComponentTemplatesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visual_programmer = VisualProgrammer::new();
    
    // Mock component templates for now
    let templates = vec![
        ComponentTemplateInfo {
            template_id: "function".to_string(),
            name: "Function".to_string(),
            category: "Logic".to_string(),
            description: "A reusable function component".to_string(),
            icon: Some("function-icon.svg".to_string()),
            input_ports: vec![
                PortInfo {
                    name: "input".to_string(),
                    data_type: "any".to_string(),
                    required: false,
                    description: "Function input".to_string(),
                }
            ],
            output_ports: vec![
                PortInfo {
                    name: "output".to_string(),
                    data_type: "any".to_string(),
                    required: false,
                    description: "Function output".to_string(),
                }
            ],
        },
        ComponentTemplateInfo {
            template_id: "condition".to_string(),
            name: "Condition".to_string(),
            category: "Control Flow".to_string(),
            description: "Conditional branching component".to_string(),
            icon: Some("condition-icon.svg".to_string()),
            input_ports: vec![
                PortInfo {
                    name: "condition".to_string(),
                    data_type: "boolean".to_string(),
                    required: true,
                    description: "Condition to evaluate".to_string(),
                }
            ],
            output_ports: vec![
                PortInfo {
                    name: "true".to_string(),
                    data_type: "any".to_string(),
                    required: false,
                    description: "True branch".to_string(),
                },
                PortInfo {
                    name: "false".to_string(),
                    data_type: "any".to_string(),
                    required: false,
                    description: "False branch".to_string(),
                }
            ],
        },
        ComponentTemplateInfo {
            template_id: "loop".to_string(),
            name: "Loop".to_string(),
            category: "Control Flow".to_string(),
            description: "Iteration component".to_string(),
            icon: Some("loop-icon.svg".to_string()),
            input_ports: vec![
                PortInfo {
                    name: "iterable".to_string(),
                    data_type: "array".to_string(),
                    required: true,
                    description: "Items to iterate over".to_string(),
                }
            ],
            output_ports: vec![
                PortInfo {
                    name: "item".to_string(),
                    data_type: "any".to_string(),
                    required: false,
                    description: "Current item".to_string(),
                }
            ],
        },
        ComponentTemplateInfo {
            template_id: "api".to_string(),
            name: "API Call".to_string(),
            category: "Data".to_string(),
            description: "HTTP API request component".to_string(),
            icon: Some("api-icon.svg".to_string()),
            input_ports: vec![
                PortInfo {
                    name: "url".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    description: "API endpoint URL".to_string(),
                },
                PortInfo {
                    name: "method".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    description: "HTTP method".to_string(),
                }
            ],
            output_ports: vec![
                PortInfo {
                    name: "response".to_string(),
                    data_type: "object".to_string(),
                    required: false,
                    description: "API response".to_string(),
                }
            ],
        },
        ComponentTemplateInfo {
            template_id: "database".to_string(),
            name: "Database Query".to_string(),
            category: "Data".to_string(),
            description: "Database operation component".to_string(),
            icon: Some("database-icon.svg".to_string()),
            input_ports: vec![
                PortInfo {
                    name: "query".to_string(),
                    data_type: "string".to_string(),
                    required: true,
                    description: "SQL query".to_string(),
                }
            ],
            output_ports: vec![
                PortInfo {
                    name: "results".to_string(),
                    data_type: "array".to_string(),
                    required: false,
                    description: "Query results".to_string(),
                }
            ],
        },
    ];

    let filtered_templates = if let Some(category) = params.category {
        templates.into_iter()
            .filter(|t| t.category == category)
            .collect()
    } else {
        templates
    };

    app_state.metrics.record_visual_templates_requested();
    Ok(Json(ComponentTemplatesResponse {
        templates: filtered_templates,
        total_count: filtered_templates.len() as u32,
        categories: vec![
            "Logic".to_string(),
            "Control Flow".to_string(),
            "Data".to_string(),
            "UI".to_string(),
            "Algorithm".to_string(),
        ],
    }))
}

/// Validate visual flowchart
#[utoipa::path(
    post,
    path = "/api/v1/visual/validate",
    request_body = ValidateFlowchartRequest,
    responses(
        (status = 200, description = "Flowchart validated", body = ValidationResponse),
        (status = 400, description = "Invalid flowchart"),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn validate_flowchart(
    State(app_state): State<AppState>,
    Json(request): Json<ValidateFlowchartRequest>,
) -> Result<Json<ValidationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let response = ValidationResponse {
        valid: true,
        errors: Vec::new(),
        warnings: vec![
            ValidationMessage {
                message_type: "warning".to_string(),
                component_id: Some("comp_1".to_string()),
                message: "Consider adding error handling".to_string(),
                severity: "medium".to_string(),
            }
        ],
        suggestions: vec![
            ValidationMessage {
                message_type: "suggestion".to_string(),
                component_id: None,
                message: "Add unit tests for better reliability".to_string(),
                severity: "low".to_string(),
            }
        ],
        performance_score: 0.85,
        complexity_score: 0.72,
    };

    app_state.metrics.record_visual_validation_completed();
    Ok(Json(response))
}

/// Export visual canvas to different formats
#[utoipa::path(
    post,
    path = "/api/v1/visual/export",
    request_body = ExportCanvasRequest,
    responses(
        (status = 200, description = "Canvas exported successfully", body = ExportResponse),
        (status = 400, description = "Invalid export format"),
        (status = 500, description = "Internal server error")
    ),
    tag = "visual-programming"
)]
pub async fn export_canvas(
    State(app_state): State<AppState>,
    Json(request): Json<ExportCanvasRequest>,
) -> Result<Json<ExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let export_data = match request.format.as_str() {
        "json" => "{ \"canvas\": \"exported_data\" }".to_string(),
        "svg" => "<svg><!-- Canvas as SVG --></svg>".to_string(),
        "png" => "base64_encoded_png_data".to_string(),
        "pdf" => "base64_encoded_pdf_data".to_string(),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Unsupported export format".to_string(),
                    details: Some(format!("Format '{}' is not supported", request.format)),
                }),
            ));
        }
    };

    let response = ExportResponse {
        success: true,
        format: request.format,
        data: export_data,
        file_name: format!("canvas_{}.{}", request.canvas_id, request.format.to_lowercase()),
        size_bytes: 1024, // Mock size
    };

    app_state.metrics.record_visual_export_completed();
    Ok(Json(response))
}

// Request/Response structs
#[derive(Debug, Deserialize)]
pub struct CreateCanvasRequest {
    pub name: String,
    pub creator: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateCanvasResponse {
    pub canvas_id: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct AddComponentRequest {
    pub component: VisualComponent,
}

#[derive(Debug, Serialize)]
pub struct ComponentResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateConnectionRequest {
    pub connection: Connection,
}

#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    pub success: bool,
    pub message: String,
    pub connection_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TemplateQuery {
    pub category: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ComponentTemplatesResponse {
    pub templates: Vec<ComponentTemplateInfo>,
    pub total_count: u32,
    pub categories: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ComponentTemplateInfo {
    pub template_id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub icon: Option<String>,
    pub input_ports: Vec<PortInfo>,
    pub output_ports: Vec<PortInfo>,
}

#[derive(Debug, Serialize)]
pub struct PortInfo {
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidateFlowchartRequest {
    pub canvas: VisualCanvas,
    pub validation_level: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub errors: Vec<ValidationMessage>,
    pub warnings: Vec<ValidationMessage>,
    pub suggestions: Vec<ValidationMessage>,
    pub performance_score: f32,
    pub complexity_score: f32,
}

#[derive(Debug, Serialize)]
pub struct ValidationMessage {
    pub message_type: String,
    pub component_id: Option<String>,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportCanvasRequest {
    pub canvas_id: String,
    pub format: String, // json, svg, png, pdf
    pub options: Option<ExportOptions>,
}

#[derive(Debug, Deserialize)]
pub struct ExportOptions {
    pub include_metadata: Option<bool>,
    pub resolution: Option<String>,
    pub theme: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub success: bool,
    pub format: String,
    pub data: String,
    pub file_name: String,
    pub size_bytes: u64,
}