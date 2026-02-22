mod create;
mod format;
mod helpers;
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
