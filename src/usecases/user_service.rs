//! Admin-only user management. There is no public sign-up: every account
//! is created here (or seeded on first start by `ensure_admin_exists`).

use crate::domain::models::{Role, User};
use crate::domain::repository::{NewUser, RepoError, UniqueViolation, UserRepo, UserUpdate};
use crate::usecases::auth_service::hash_password;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub enum UserServiceError {
    InvalidRole(String),
    InvalidEmail(String),
    UsernameTaken,
    EmailTaken,
    NotFound,
    CannotDeleteSelf,
    /// The change would leave nobody able to manage users.
    LastAdmin,
    Repo(RepoError),
}

impl fmt::Display for UserServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserServiceError::InvalidRole(r) => write!(f, "invalid role: {r}"),
            UserServiceError::InvalidEmail(e) => write!(f, "invalid email: {e}"),
            UserServiceError::UsernameTaken => write!(f, "username already exists"),
            UserServiceError::EmailTaken => write!(f, "email already exists"),
            UserServiceError::NotFound => write!(f, "user not found"),
            UserServiceError::CannotDeleteSelf => write!(f, "cannot delete your own account"),
            UserServiceError::LastAdmin => write!(f, "there must always be at least one admin"),
            UserServiceError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UserServiceError {}

impl From<RepoError> for UserServiceError {
    fn from(e: RepoError) -> Self {
        match e.downcast_ref::<UniqueViolation>() {
            Some(v) if v.constraint.contains("username") => UserServiceError::UsernameTaken,
            Some(v) if v.constraint.contains("email") => UserServiceError::EmailTaken,
            _ => UserServiceError::Repo(e),
        }
    }
}

// Deliberately not a full RFC 5322 validator — just enough to catch
// obvious typos ("no @", "no domain") without a regex dependency.
fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains(' ')
}

fn check_role(role: &str) -> Result<Role, UserServiceError> {
    Role::parse(role).ok_or_else(|| UserServiceError::InvalidRole(role.to_string()))
}

fn check_email(email: &str) -> Result<(), UserServiceError> {
    if is_valid_email(email) {
        Ok(())
    } else {
        Err(UserServiceError::InvalidEmail(email.to_string()))
    }
}

fn hash(password: &str) -> Result<String, UserServiceError> {
    hash_password(password).map_err(|e| UserServiceError::Repo(e.to_string().into()))
}

pub struct CreateUser<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub role: &'a str,
    pub email: &'a str,
    pub phone: Option<&'a str>,
}

/// A partial update: only the `Some` fields change.
#[derive(Default)]
pub struct UpdateUser<'a> {
    pub role: Option<&'a str>,
    pub password: Option<&'a str>,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
}

pub struct UserService {
    repo: Arc<dyn UserRepo>,
}

impl UserService {
    pub fn new(repo: Arc<dyn UserRepo>) -> Self {
        Self { repo }
    }

    pub async fn list(&self) -> Result<Vec<User>, UserServiceError> {
        Ok(self.repo.list_users().await?)
    }

    pub async fn find(&self, id: i64) -> Result<User, UserServiceError> {
        self.repo
            .find_user_by_id(id)
            .await?
            .ok_or(UserServiceError::NotFound)
    }

    pub async fn create(&self, input: CreateUser<'_>) -> Result<User, UserServiceError> {
        check_role(input.role)?;
        check_email(input.email)?;
        let password_hash = hash(input.password)?;
        let user = self
            .repo
            .create_user(NewUser {
                username: input.username,
                password_hash: &password_hash,
                role: input.role,
                email: input.email,
                phone: input.phone,
            })
            .await?;
        Ok(user)
    }

    pub async fn update(&self, id: i64, input: UpdateUser<'_>) -> Result<(), UserServiceError> {
        if let Some(role) = input.role {
            if check_role(role)? != Role::Admin {
                self.ensure_not_last_admin(id).await?;
            }
        }
        if let Some(email) = input.email {
            check_email(email)?;
        }
        let password_hash = input.password.map(hash).transpose()?;

        let found = self
            .repo
            .update_user(
                id,
                UserUpdate {
                    role: input.role,
                    password_hash: password_hash.as_deref(),
                    email: input.email,
                    phone: input.phone,
                },
            )
            .await?;
        if !found {
            return Err(UserServiceError::NotFound);
        }
        Ok(())
    }

    /// `acting_user_id` is the admin making the request.
    pub async fn delete(&self, acting_user_id: i64, id: i64) -> Result<(), UserServiceError> {
        if acting_user_id == id {
            return Err(UserServiceError::CannotDeleteSelf);
        }
        self.ensure_not_last_admin(id).await?;
        if !self.repo.delete_user(id).await? {
            return Err(UserServiceError::NotFound);
        }
        Ok(())
    }

    /// Fails if `id` is the only admin, so removing or demoting them would
    /// lock everyone out of user management.
    async fn ensure_not_last_admin(&self, id: i64) -> Result<(), UserServiceError> {
        let admins: Vec<User> = self
            .repo
            .list_users()
            .await?
            .into_iter()
            .filter(|u| u.role == Role::Admin.as_str())
            .collect();
        if admins.len() == 1 && admins[0].id == Some(id) {
            return Err(UserServiceError::LastAdmin);
        }
        Ok(())
    }

    /// On first start (no users at all), creates the admin account from the
    /// given credentials. Returns whether an account was created.
    pub async fn ensure_admin_exists(
        &self,
        username: &str,
        password: &str,
        email: &str,
    ) -> Result<bool, UserServiceError> {
        if self.repo.count_users().await? > 0 {
            return Ok(false);
        }
        self.create(CreateUser {
            username,
            password,
            role: Role::Admin.as_str(),
            email,
            phone: None,
        })
        .await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::sqlite::test_repo;

    async fn service() -> UserService {
        UserService::new(test_repo().await)
    }

    fn new_user<'a>(username: &'a str, role: &'a str, email: &'a str) -> CreateUser<'a> {
        CreateUser {
            username,
            password: "password123",
            role,
            email,
            phone: None,
        }
    }

    async fn create(service: &UserService, username: &str, role: &str) -> i64 {
        let email = format!("{username}@example.com");
        service
            .create(new_user(username, role, &email))
            .await
            .unwrap()
            .id
            .unwrap()
    }

    #[tokio::test]
    async fn create_validates_role_and_email_before_writing() {
        let service = service().await;
        let bad_role = service
            .create(new_user("alice", "superuser", "alice@example.com"))
            .await;
        assert!(matches!(bad_role, Err(UserServiceError::InvalidRole(_))));

        for email in ["not-an-email", "alice@localhost", "@example.com", "a@.com"] {
            let result = service.create(new_user("alice", "user", email)).await;
            assert!(
                matches!(result, Err(UserServiceError::InvalidEmail(_))),
                "{email}"
            );
        }
        assert!(service.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_hashes_the_password() {
        let service = service().await;
        let user = service
            .create(new_user("alice", "admin", "alice@example.com"))
            .await
            .unwrap();
        assert_ne!(user.password_hash, "password123");
    }

    #[tokio::test]
    async fn duplicate_username_and_email_are_typed_errors() {
        let service = service().await;
        service
            .create(new_user("alice", "user", "alice@example.com"))
            .await
            .unwrap();

        let same_name = service
            .create(new_user("alice", "user", "other@example.com"))
            .await;
        assert!(matches!(same_name, Err(UserServiceError::UsernameTaken)));
        let same_email = service
            .create(new_user("bob", "user", "alice@example.com"))
            .await;
        assert!(matches!(same_email, Err(UserServiceError::EmailTaken)));
    }

    #[tokio::test]
    async fn update_changes_only_the_given_fields() {
        let service = service().await;
        let id = create(&service, "carol", "manager").await;
        let before = service.find(id).await.unwrap();

        service
            .update(
                id,
                UpdateUser {
                    password: Some("new-password"),
                    phone: Some("+1-555-0100"),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let after = service.find(id).await.unwrap();
        assert_eq!(after.role, "manager");
        assert_eq!(after.email, before.email);
        assert_eq!(after.phone.as_deref(), Some("+1-555-0100"));
        assert_ne!(after.password_hash, before.password_hash);
        assert_ne!(after.password_hash, "new-password");
    }

    #[tokio::test]
    async fn update_rejects_bad_input_and_unknown_users() {
        let service = service().await;
        let id = create(&service, "bob", "user").await;

        let bad_role = UpdateUser {
            role: Some("superuser"),
            ..Default::default()
        };
        assert!(matches!(
            service.update(id, bad_role).await,
            Err(UserServiceError::InvalidRole(_))
        ));
        let bad_email = UpdateUser {
            email: Some("nope"),
            ..Default::default()
        };
        assert!(matches!(
            service.update(id, bad_email).await,
            Err(UserServiceError::InvalidEmail(_))
        ));
        assert!(matches!(
            service.update(9999, UpdateUser::default()).await,
            Err(UserServiceError::NotFound)
        ));
        assert_eq!(service.find(id).await.unwrap().role, "user");
    }

    #[tokio::test]
    async fn the_last_admin_cannot_be_demoted_but_one_of_two_can() {
        let service = service().await;
        let admin = create(&service, "admin", "admin").await;
        let demote = || UpdateUser {
            role: Some("user"),
            ..Default::default()
        };

        assert!(matches!(
            service.update(admin, demote()).await,
            Err(UserServiceError::LastAdmin)
        ));

        create(&service, "admin2", "admin").await;
        service.update(admin, demote()).await.unwrap();
        assert_eq!(service.find(admin).await.unwrap().role, "user");
    }

    #[tokio::test]
    async fn delete_refuses_self_the_last_admin_and_unknown_users() {
        let service = service().await;
        let admin = create(&service, "admin", "admin").await;
        let manager = create(&service, "manager", "manager").await;

        assert!(matches!(
            service.delete(admin, admin).await,
            Err(UserServiceError::CannotDeleteSelf)
        ));
        assert!(matches!(
            service.delete(manager, admin).await,
            Err(UserServiceError::LastAdmin)
        ));
        assert!(matches!(
            service.delete(admin, 9999).await,
            Err(UserServiceError::NotFound)
        ));

        service.delete(admin, manager).await.unwrap();
        assert!(matches!(
            service.find(manager).await,
            Err(UserServiceError::NotFound)
        ));
    }

    #[tokio::test]
    async fn ensure_admin_exists_only_seeds_an_empty_user_table() {
        let service = service().await;
        assert!(
            service
                .ensure_admin_exists("root", "pw", "root@example.com")
                .await
                .unwrap()
        );
        assert!(
            !service
                .ensure_admin_exists("root2", "pw", "root2@example.com")
                .await
                .unwrap()
        );
        let users = service.list().await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].role, "admin");
    }
}
