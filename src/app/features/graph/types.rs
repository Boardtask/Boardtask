/// Path parameters for node endpoints with ID.
#[derive(Debug, serde::Deserialize)]
pub struct NodePathParams {
    pub project_id: String,
    pub id: String,
}