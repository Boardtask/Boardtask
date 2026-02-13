use askama::Template;
use axum::{routing::get, Router};

use crate::app::{AppState, APP_NAME};

/// The home page template.
#[derive(Template)]
#[template(path = "site/home.html")]
pub struct HomeTemplate {
    pub app_name: &'static str,
}

/// GET /
pub async fn index() -> HomeTemplate {
    HomeTemplate {
        app_name: APP_NAME,
    }
}

/// Routes for the home feature slice.
pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(index))
}