use axum::{Router, extract::State, response::Json, routing::get};

use crate::AppState;
use crate::error::AppError;
use crate::model::AllocationResponse;
use crate::services::allocations::build_allocation_snapshot;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/allocations", get(get_allocations))
        .with_state(state)
}

async fn get_allocations(
    State(state): State<AppState>,
) -> Result<Json<AllocationResponse>, AppError> {
    let snapshot = build_allocation_snapshot(&state).await?;
    Ok(Json(snapshot))
}
