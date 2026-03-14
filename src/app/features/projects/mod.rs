mod create;
mod delete;
mod export;
mod format;
mod helpers;
mod import;
mod import_export;
mod list;
mod list_view;
mod progress;
mod show;
mod update_settings;

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

/// API routes for projects (export, import, delete, update settings) under /api/projects/...
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(export::routes())
        .merge(import::routes())
        .merge(delete::routes())
        .merge(update_settings::routes())
}
