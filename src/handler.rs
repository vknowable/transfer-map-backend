use std::sync::Arc;

use axum::{
  extract::{Query, State},
  http::StatusCode,
  response::IntoResponse,
  Json,
};

use crate::{
  model::TxdataModel,
  schema::FilterOptions,
  AppState,
};

pub async fn health_checker_handler() -> impl IntoResponse {
  let json_response = serde_json::json!({
      "code": 200,
      "success": true,
      "payload": {
          "message": "Service is healthy."
      }
  });

  Json(json_response)
}


pub async fn txdata_list_handler(
  opts: Option<Query<FilterOptions>>,
  State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
  let Query(opts) = opts.unwrap_or_default();

  let limit = opts.limit.unwrap_or(100);
  let offset = (opts.page.unwrap_or(1) - 1) * limit;

  let query_result = sqlx::query_as!(
    TxdataModel,
    "SELECT * FROM txdata ORDER by txhash LIMIT $1 OFFSET $2",
    limit as i32,
    offset as i32
  )
  .fetch_all(&data.db)
  .await;

  if query_result.is_err() {
    let error_response = serde_json::json!({
      "status": "fail",
      "message": "Error occurred while fetching all tx entries."
    });
    return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
  }

  let txdata = query_result.unwrap();

  let json_response = serde_json::json!({
    "status": "success",
    "results": txdata.len(),
    "txdata": txdata
  });
  Ok(Json(json_response))
}