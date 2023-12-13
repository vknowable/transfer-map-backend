use std::sync::Arc;

use axum::{
  routing::get,
  Router,
};

use crate::{
  handler::{txdata_list_handler, health_checker_handler},
  AppState,
};

pub fn create_router(app_state: Arc<AppState>) -> Router {
  Router::new()
  .route("/api/healthchecker", get(health_checker_handler))
  .route("/api/txdata", get(txdata_list_handler))
  .with_state(app_state)
}