use axum::{
    routing::post,
    Router,
    Json,
};
use serde::{Deserialize, Serialize};
use fql_parser::parse;
use fql_engine::{execute_query, FileRecord};

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<FileRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

async fn handle_query(Json(payload): Json<QueryRequest>) -> Json<QueryResponse> {
    match parse(&payload.query) {
        Ok(ast) => {
            match execute_query(ast) {
                Ok(records) => Json(QueryResponse {
                    success: true,
                    data: Some(records),
                    error: None,
                }),
                Err(e) => Json(QueryResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                }),
            }
        },
        Err(e) => Json(QueryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()), 
        }),
    }
}

pub async fn start_server(port: u16) {
    let app = Router::new()
        .route("/query", post(handle_query));

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
