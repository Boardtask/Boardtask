mod create;
mod export;
mod format;
mod helpers;
mod import;
mod import_export;
mod list;
mod list_view;
mod progress;
mod show;

use axum::Router;

use crate::app::AppState;

/// Projects routes (list, create, show, list_view).
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(list::routes())
        .merge(create::routes())
        .merge(show::routes())
        .merge(list_view::routes())
}

/// API routes for projects (export, import) under /api/projects/...
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(export::routes())
        .merge(import::routes())
}
