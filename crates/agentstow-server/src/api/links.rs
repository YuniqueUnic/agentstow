use std::sync::Arc;

use agentstow_web_types::{LinkApplyRequest, LinkPlanRequest, LinkRepairRequest};
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as JsonResp, Response};

use crate::ServerState;
use crate::api::helpers::{api_error, handle_result, queries_from_state};

pub(super) async fn api_links(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.link_records() {
        Ok(records) => JsonResp(records).into_response(),
        Err(error) => api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

pub(super) async fn api_links_plan(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<LinkPlanRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.link_plan(req))
}

pub(super) async fn api_links_apply(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<LinkApplyRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.link_apply(req))
}

pub(super) async fn api_links_repair(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<LinkRepairRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.link_repair(req))
}

pub(super) async fn api_link_status(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.link_status() {
        Ok(status) => JsonResp(status).into_response(),
        Err(error) => api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}
