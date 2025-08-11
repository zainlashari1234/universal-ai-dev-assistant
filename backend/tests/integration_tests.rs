use std::time::Duration;
use tokio::time::timeout;
use serde_json::json;

#[tokio::test]
async fn test_health_endpoint() {
    let response = reqwest::get("http://127.0.0.1:8080/health").await;
    assert!(response.is_ok());
    
    let body = response.unwrap().json::<serde_json::Value>().await.unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_code_completion() {
    let client = reqwest::Client::new();
    let request_body = json!({
        "code": "def fibonacci(n):\n    if n <= 1:\n        return n\n    return ",
        "language": "python",
        "cursor_position": 65
    });
    
    let response = client
        .post("http://127.0.0.1:8080/api/v1/complete")
        .json(&request_body)
        .send()
        .await;
    
    assert!(response.is_ok());
    let body = response.unwrap().json::<serde_json::Value>().await.unwrap();
    assert!(body["suggestions"].is_array());
}

#[tokio::test]
async fn test_code_analysis() {
    let client = reqwest::Client::new();
    let request_body = json!({
        "code": "password = 'admin123'\neval('print(\"hello\")')",
        "language": "python",
        "cursor_position": 0
    });
    
    let response = client
        .post("http://127.0.0.1:8080/api/v1/analyze")
        .json(&request_body)
        .send()
        .await;
    
    assert!(response.is_ok());
    let body = response.unwrap().json::<serde_json::Value>().await.unwrap();
    assert!(body["issues"].is_array());
}

#[tokio::test]
async fn test_response_time() {
    let client = reqwest::Client::new();
    let request_body = json!({
        "code": "print('hello world')",
        "language": "python",
        "cursor_position": 0
    });
    
    let start = std::time::Instant::now();
    let response = client
        .post("http://127.0.0.1:8080/api/v1/complete")
        .json(&request_body)
        .send()
        .await;
    let duration = start.elapsed();
    
    assert!(response.is_ok());
    assert!(duration < Duration::from_millis(1000)); // Should respond within 1 second
}