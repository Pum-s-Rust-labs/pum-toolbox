//! Hệ thống phân quyền đa tầng (3-Tier RBAC):
//! 1. Phân cấp vai trò hệ thống (`SuperAdmin > Admin > Member > Viewer`).
//! 2. Quyền hạt mịn (`Permission`) kèm hỗ trợ cấp/thu hồi riêng cho từng user.
//! 3. Kiểm soát theo quyền sở hữu (`owner_id`) và mức hiển thị (`Visibility`).

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// 4 cấp vai trò hệ thống theo thứ tự phân cấp rõ ràng.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Viewer = 1,
    Member = 2,
    Admin = 3,
    SuperAdmin = 4,
}

impl Role {
    /// Trả về chuỗi định danh lưu trong DB và JWT.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Member => "member",
            Self::Admin => "admin",
            Self::SuperAdmin => "super_admin",
        }
    }

    /// Phân tích chuỗi thành `Role`.
    pub fn parse(raw: &str) -> AppResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "viewer" => Ok(Self::Viewer),
            "member" => Ok(Self::Member),
            "admin" => Ok(Self::Admin),
            "super_admin" | "superadmin" => Ok(Self::SuperAdmin),
            other => Err(AppError::Validation(format!(
                "vai trò `{other}` không hợp lệ (chỉ hỗ trợ super_admin, admin, member, viewer)"
            ))),
        }
    }

    /// Cấp bậc số của vai trò (càng cao quyền càng lớn).
    #[must_use]
    pub const fn tier(self) -> u8 {
        self as u8
    }

    /// Kiểm tra vai trò hiện tại có được phép quản lý/đổi vai trò của `target` không.
    /// Quy tắc: chỉ `SuperAdmin` được gán/thu hồi `Admin`/`SuperAdmin`;
    /// `Admin` chỉ quản lý được `Member` và `Viewer`.
    #[must_use]
    pub const fn can_manage_role(self, target: Self) -> bool {
        match self {
            Self::SuperAdmin => true,
            Self::Admin => matches!(target, Self::Member | Self::Viewer),
            Self::Member | Self::Viewer => false,
        }
    }

    /// Tập quyền mặc định tương ứng với mỗi vai trò.
    #[must_use]
    pub fn default_permissions(self) -> BTreeSet<Permission> {
        use Permission::{
            AuditRead, ImagesProcess, LinksDeleteAny, LinksRead, LinksWrite, MonitorsDeleteAny,
            MonitorsRead, MonitorsWrite, PastesDeleteAny, PastesRead, PastesWrite, RolesAssign,
            UsersManage,
        };

        let mut perms = BTreeSet::new();
        // Tầng Viewer: chỉ đọc tài nguyên công khai/nội bộ
        perms.insert(LinksRead);
        perms.insert(MonitorsRead);
        perms.insert(PastesRead);

        if self >= Self::Member {
            perms.insert(LinksWrite);
            perms.insert(MonitorsWrite);
            perms.insert(PastesWrite);
            perms.insert(ImagesProcess);
        }

        if self >= Self::Admin {
            perms.insert(LinksDeleteAny);
            perms.insert(MonitorsDeleteAny);
            perms.insert(PastesDeleteAny);
            perms.insert(UsersManage);
            perms.insert(AuditRead);
        }

        if self >= Self::SuperAdmin {
            perms.insert(RolesAssign);
        }

        perms
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Các quyền hạt mịn (granular permissions) trên toàn bộ 4 module và khu vực quản trị.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "links:read")]
    LinksRead,
    #[serde(rename = "links:write")]
    LinksWrite,
    #[serde(rename = "links:delete_any")]
    LinksDeleteAny,
    #[serde(rename = "monitors:read")]
    MonitorsRead,
    #[serde(rename = "monitors:write")]
    MonitorsWrite,
    #[serde(rename = "monitors:delete_any")]
    MonitorsDeleteAny,
    #[serde(rename = "pastes:read")]
    PastesRead,
    #[serde(rename = "pastes:write")]
    PastesWrite,
    #[serde(rename = "pastes:delete_any")]
    PastesDeleteAny,
    #[serde(rename = "images:process")]
    ImagesProcess,
    #[serde(rename = "users:manage")]
    UsersManage,
    #[serde(rename = "roles:assign")]
    RolesAssign,
    #[serde(rename = "audit:read")]
    AuditRead,
}

impl Permission {
    /// Danh sách đầy đủ tất cả quyền hợp lệ trong hệ thống.
    pub const ALL: &'static [Self] = &[
        Self::LinksRead,
        Self::LinksWrite,
        Self::LinksDeleteAny,
        Self::MonitorsRead,
        Self::MonitorsWrite,
        Self::MonitorsDeleteAny,
        Self::PastesRead,
        Self::PastesWrite,
        Self::PastesDeleteAny,
        Self::ImagesProcess,
        Self::UsersManage,
        Self::RolesAssign,
        Self::AuditRead,
    ];

    /// Trả về chuỗi mã quyền chuẩn `resource:action`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinksRead => "links:read",
            Self::LinksWrite => "links:write",
            Self::LinksDeleteAny => "links:delete_any",
            Self::MonitorsRead => "monitors:read",
            Self::MonitorsWrite => "monitors:write",
            Self::MonitorsDeleteAny => "monitors:delete_any",
            Self::PastesRead => "pastes:read",
            Self::PastesWrite => "pastes:write",
            Self::PastesDeleteAny => "pastes:delete_any",
            Self::ImagesProcess => "images:process",
            Self::UsersManage => "users:manage",
            Self::RolesAssign => "roles:assign",
            Self::AuditRead => "audit:read",
        }
    }

    /// Phân tích chuỗi `resource:action` thành `Permission`.
    pub fn parse(raw: &str) -> AppResult<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|p| p.as_str().eq_ignore_ascii_case(raw.trim()))
            .ok_or_else(|| AppError::Validation(format!("quyền `{raw}` không tồn tại")))
    }
}

/// Mức độ hiển thị của một tài nguyên (link, monitor, paste).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Private,
    #[default]
    Internal,
    Public,
}

impl Visibility {
    /// Chuỗi lưu trong DB.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Internal => "internal",
            Self::Public => "public",
        }
    }

    /// Phân tích từ chuỗi.
    pub fn parse(raw: &str) -> AppResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "private" => Ok(Self::Private),
            "internal" => Ok(Self::Internal),
            "public" => Ok(Self::Public),
            other => Err(AppError::Validation(format!(
                "visibility `{other}` không hợp lệ (chỉ hỗ trợ private, internal, public)"
            ))),
        }
    }
}

/// Bản ghi ghi đè quyền riêng lẻ cho một user cụ thể.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOverride {
    pub permission: Permission,
    pub granted: bool,
}

/// Tính tập quyền hiệu lực cuối cùng từ Role + danh sách override của user.
#[must_use]
pub fn resolve_permissions(role: Role, overrides: &[PermissionOverride]) -> BTreeSet<Permission> {
    let mut perms = role.default_permissions();
    for item in overrides {
        if item.granted {
            perms.insert(item.permission);
        } else {
            perms.remove(&item.permission);
        }
    }
    perms
}

/// Danh tính người dùng đã xác thực kèm tập quyền hiệu lực trong request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub user_id: Uuid,
    pub email: String,
    pub username: String,
    pub role: Role,
    pub permissions: BTreeSet<Permission>,
}

impl Principal {
    /// Kiểm tra có quyền cụ thể không.
    #[must_use]
    pub fn has(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Bắt buộc phải có quyền, nếu không trả `AppError::Forbidden`.
    pub fn require(&self, permission: Permission) -> AppResult<()> {
        if self.has(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "bạn không có quyền `{}` để thực hiện thao tác này",
                permission.as_str()
            )))
        }
    }

    /// Kiểm tra quyền đọc tài nguyên dựa trên `Visibility` và `owner_id`.
    #[must_use]
    pub fn can_read_resource(
        &self,
        owner_id: Uuid,
        visibility: Visibility,
        read_perm: Permission,
        admin_override_perm: Permission,
    ) -> bool {
        if !self.has(read_perm) {
            return false;
        }
        match visibility {
            Visibility::Public | Visibility::Internal => true,
            Visibility::Private => owner_id == self.user_id || self.has(admin_override_perm),
        }
    }

    /// Kiểm tra quyền sửa/xóa tài nguyên (chủ sở hữu có `write_perm`, hoặc quản trị có `delete_any_perm`).
    pub fn require_mutate_resource(
        &self,
        owner_id: Uuid,
        write_perm: Permission,
        admin_override_perm: Permission,
    ) -> AppResult<()> {
        if self.has(admin_override_perm) {
            return Ok(());
        }
        if owner_id == self.user_id && self.has(write_perm) {
            return Ok(());
        }
        Err(AppError::Forbidden(
            "bạn chỉ có thể chỉnh sửa hoặc xóa tài nguyên do chính mình tạo".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_hierarchy_enforces_management_rules() {
        assert!(Role::SuperAdmin.can_manage_role(Role::Admin));
        assert!(Role::SuperAdmin.can_manage_role(Role::SuperAdmin));
        assert!(Role::Admin.can_manage_role(Role::Member));
        assert!(Role::Admin.can_manage_role(Role::Viewer));
        assert!(!Role::Admin.can_manage_role(Role::Admin));
        assert!(!Role::Admin.can_manage_role(Role::SuperAdmin));
        assert!(!Role::Member.can_manage_role(Role::Viewer));
    }

    #[test]
    fn permission_overrides_can_grant_and_revoke() {
        let overrides = vec![
            PermissionOverride {
                permission: Permission::ImagesProcess,
                granted: true,
            },
            PermissionOverride {
                permission: Permission::LinksRead,
                granted: false,
            },
        ];
        let perms = resolve_permissions(Role::Viewer, &overrides);
        assert!(perms.contains(&Permission::ImagesProcess));
        assert!(!perms.contains(&Permission::LinksRead));
        assert!(perms.contains(&Permission::MonitorsRead));
    }

    #[test]
    fn ownership_and_visibility_enforced_properly() {
        let alice_id = Uuid::new_v4();
        let bob_id = Uuid::new_v4();

        let alice = Principal {
            user_id: alice_id,
            email: "alice@example.com".to_owned(),
            username: "alice".to_owned(),
            role: Role::Member,
            permissions: Role::Member.default_permissions(),
        };

        let admin = Principal {
            user_id: Uuid::new_v4(),
            email: "admin@example.com".to_owned(),
            username: "admin".to_owned(),
            role: Role::Admin,
            permissions: Role::Admin.default_permissions(),
        };

        // Alice can read & mutate her own private resource, but not Bob's private resource
        assert!(alice.can_read_resource(
            alice_id,
            Visibility::Private,
            Permission::PastesRead,
            Permission::PastesDeleteAny,
        ));
        assert!(!alice.can_read_resource(
            bob_id,
            Visibility::Private,
            Permission::PastesRead,
            Permission::PastesDeleteAny,
        ));
        assert!(
            alice
                .require_mutate_resource(
                    bob_id,
                    Permission::PastesWrite,
                    Permission::PastesDeleteAny
                )
                .is_err()
        );

        // Admin can override read & mutate
        assert!(admin.can_read_resource(
            bob_id,
            Visibility::Private,
            Permission::PastesRead,
            Permission::PastesDeleteAny,
        ));
        assert!(
            admin
                .require_mutate_resource(
                    bob_id,
                    Permission::PastesWrite,
                    Permission::PastesDeleteAny
                )
                .is_ok()
        );
    }
}
