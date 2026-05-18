use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct PermissionRequest {
    #[serde(rename = "id_feature")]
    pub feature: String,
    pub create: bool,
    pub view: bool,
    pub activate: bool,
    pub delete: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<PermissionRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Option<Vec<PermissionRequest>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub permissions: Vec<PermissionRequest>,
}
