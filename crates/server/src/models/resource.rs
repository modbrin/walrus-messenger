use crate::models::user::UserId;

pub type ResourceId = i64;

#[derive(Clone, Debug)]
pub struct CreateResourceRequest {
    pub uploaded_by_user_id: Option<UserId>,
    pub url: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Resource {
    pub id: ResourceId,
    pub uploaded_by_user_id: Option<UserId>,
    pub url: String,
}
