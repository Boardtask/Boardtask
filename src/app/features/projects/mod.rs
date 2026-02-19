mod create;
mod list;
mod show;

use axum::Router;

use crate::app::AppState;

/// Projects routes (list, create, show).
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(list::routes())
        .merge(create::routes())
        .merge(show::routes())
}
