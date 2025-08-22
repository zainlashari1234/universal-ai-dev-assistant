// Role-Based Access Control module
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Developer,
    Admin,
    SuperAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePermissions {
    pub role: Role,
    pub permissions: Vec<Permission>,
}

impl Default for RolePermissions {
    fn default() -> Self {
        Self {
            role: Role::User,
            permissions: vec![Permission::Read],
        }
    }
}

pub fn check_permission(role: &Role, required_permission: &Permission) -> bool {
    match (role, required_permission) {
        (Role::SuperAdmin, _) => true,
        (Role::Admin, Permission::Admin) => false,
        (Role::Admin, _) => true,
        (Role::Developer, Permission::Admin) => false,
        (Role::Developer, _) => true,
        (Role::User, Permission::Read) => true,
        _ => false,
    }
}