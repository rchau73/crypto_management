use crate::domain::models::{Role, User};
use crate::domain::repository::{NewUser, RepoResult, UserRepo, UserUpdate};
use crate::usecases::auth_service::hash_password;
use std::fmt;
use std::sync::Arc;

#[derive(Debug)]
pub enum UserServiceError {
    InvalidRole(String),
    InvalidEmail(String),
    UsernameTaken,
    EmailTaken,
    Repo(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for UserServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserServiceError::InvalidRole(r) => write!(f, "invalid role: {r}"),
            UserServiceError::InvalidEmail(e) => write!(f, "invalid email: {e}"),
            UserServiceError::UsernameTaken => write!(f, "username already exists"),
            UserServiceError::EmailTaken => write!(f, "email already exists"),
            UserServiceError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UserServiceError {}

impl From<Box<dyn std::error::Error + Send + Sync>> for UserServiceError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        // sqlx surfaces a UNIQUE constraint violation as a generic DB error;
        // recognize it by message so callers get a clean 409, not a 500.
        let msg = e.to_string();
        if msg.contains("UNIQUE constraint failed: users.username") {
            UserServiceError::UsernameTaken
        } else if msg.contains("UNIQUE constraint failed") && msg.contains("email") {
            UserServiceError::EmailTaken
        } else {
            UserServiceError::Repo(e)
        }
    }
}

// Deliberately not a full RFC 5322 validator — just enough to catch obvious
// typos ("no @ at all", "no domain") without dragging in a regex dependency
// for a personal app's user list.
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

/// Admin-only CRUD over user accounts. There is no public self-registration
/// endpoint anywhere in this app — every account is created here.
pub struct UserService {
    repo: Arc<dyn UserRepo>,
}

impl UserService {
    pub fn new(repo: Arc<dyn UserRepo>) -> Self {
        Self { repo }
    }

    pub async fn list(&self) -> RepoResult<Vec<User>> {
        self.repo.list_users().await
    }

    pub async fn create(
        &self,
        username: &str,
        password: &str,
        role: &str,
        email: &str,
        phone: Option<&str>,
    ) -> Result<User, UserServiceError> {
        Role::parse(role).ok_or_else(|| UserServiceError::InvalidRole(role.to_string()))?;
        if !is_valid_email(email) {
            return Err(UserServiceError::InvalidEmail(email.to_string()));
        }
        let password_hash =
            hash_password(password).map_err(|e| UserServiceError::Repo(e.to_string().into()))?;
        self.repo
            .create_user(NewUser {
                username,
                password_hash: &password_hash,
                role,
                email,
                phone,
            })
            .await
            .map_err(Into::into)
    }

    pub async fn update(
        &self,
        id: i64,
        role: Option<&str>,
        password: Option<&str>,
        email: Option<&str>,
        phone: Option<&str>,
    ) -> Result<(), UserServiceError> {
        if let Some(r) = role {
            Role::parse(r).ok_or_else(|| UserServiceError::InvalidRole(r.to_string()))?;
        }
        if let Some(e) = email {
            if !is_valid_email(e) {
                return Err(UserServiceError::InvalidEmail(e.to_string()));
            }
        }
        let password_hash = match password {
            Some(p) => {
                Some(hash_password(p).map_err(|e| UserServiceError::Repo(e.to_string().into()))?)
            }
            None => None,
        };
        self.repo
            .update_user(
                id,
                UserUpdate {
                    role,
                    password_hash: password_hash.as_deref(),
                    email,
                    phone,
                },
            )
            .await
            .map_err(Into::into)
    }

    pub async fn delete(&self, id: i64) -> RepoResult<()> {
        self.repo.delete_user(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::RepoResult as Result_;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeUserRepo {
        users: Mutex<Vec<User>>,
        next_id: Mutex<i64>,
    }

    #[async_trait]
    impl UserRepo for FakeUserRepo {
        async fn find_user_by_username(&self, username: &str) -> Result_<Option<User>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.username == username)
                .cloned())
        }
        async fn find_user_by_id(&self, id: i64) -> Result_<Option<User>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .iter()
                .find(|u| u.id == Some(id))
                .cloned())
        }
        async fn list_users(&self) -> Result_<Vec<User>> {
            Ok(self.users.lock().unwrap().clone())
        }
        async fn count_users(&self) -> Result_<i64> {
            Ok(self.users.lock().unwrap().len() as i64)
        }
        async fn create_user(&self, new_user: NewUser<'_>) -> Result_<User> {
            let mut users = self.users.lock().unwrap();
            if users.iter().any(|u| u.username == new_user.username) {
                return Err("UNIQUE constraint failed: users.username".into());
            }
            if users.iter().any(|u| u.email == new_user.email) {
                return Err("UNIQUE constraint failed: idx_users_email (email)".into());
            }
            let mut next_id = self.next_id.lock().unwrap();
            *next_id += 1;
            let user = User {
                id: Some(*next_id),
                username: new_user.username.to_string(),
                password_hash: new_user.password_hash.to_string(),
                role: new_user.role.to_string(),
                email: new_user.email.to_string(),
                phone: new_user.phone.map(|p| p.to_string()),
                created_at: None,
                updated_at: None,
            };
            users.push(user.clone());
            Ok(user)
        }
        async fn update_user(&self, id: i64, update: UserUpdate<'_>) -> Result_<()> {
            let mut users = self.users.lock().unwrap();
            if let Some(u) = users.iter_mut().find(|u| u.id == Some(id)) {
                if let Some(r) = update.role {
                    u.role = r.to_string();
                }
                if let Some(p) = update.password_hash {
                    u.password_hash = p.to_string();
                }
                if let Some(e) = update.email {
                    u.email = e.to_string();
                }
                if let Some(p) = update.phone {
                    u.phone = Some(p.to_string());
                }
            }
            Ok(())
        }
        async fn delete_user(&self, id: i64) -> Result_<()> {
            self.users.lock().unwrap().retain(|u| u.id != Some(id));
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_rejects_an_invalid_role_before_touching_the_repo() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        let result = service
            .create(
                "alice",
                "password123",
                "superuser",
                "alice@example.com",
                None,
            )
            .await;
        assert!(matches!(result, Err(UserServiceError::InvalidRole(_))));
    }

    #[tokio::test]
    async fn create_rejects_an_email_with_no_at_sign() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        let result = service
            .create("alice", "password123", "user", "not-an-email", None)
            .await;
        assert!(matches!(result, Err(UserServiceError::InvalidEmail(_))));
    }

    #[tokio::test]
    async fn create_rejects_an_email_with_no_domain_dot() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        let result = service
            .create("alice", "password123", "user", "alice@localhost", None)
            .await;
        assert!(matches!(result, Err(UserServiceError::InvalidEmail(_))));
    }

    #[tokio::test]
    async fn create_accepts_a_well_formed_email_and_an_absent_phone() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        let user = service
            .create("alice", "password123", "user", "alice@example.com", None)
            .await
            .unwrap();
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.phone, None);
    }

    #[tokio::test]
    async fn create_hashes_the_password_rather_than_storing_it_plain() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        let user = service
            .create("alice", "password123", "admin", "alice@example.com", None)
            .await
            .unwrap();
        assert_ne!(user.password_hash, "password123");
    }

    #[tokio::test]
    async fn create_surfaces_a_duplicate_username_as_a_typed_error() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        service
            .create("alice", "password123", "user", "alice@example.com", None)
            .await
            .unwrap();
        let result = service
            .create(
                "alice",
                "different-password",
                "admin",
                "alice2@example.com",
                None,
            )
            .await;
        assert!(matches!(result, Err(UserServiceError::UsernameTaken)));
    }

    #[tokio::test]
    async fn create_surfaces_a_duplicate_email_as_a_typed_error() {
        let service = UserService::new(Arc::new(FakeUserRepo::default()));
        service
            .create("alice", "password123", "user", "shared@example.com", None)
            .await
            .unwrap();
        let result = service
            .create("alice2", "password123", "user", "shared@example.com", None)
            .await;
        assert!(matches!(result, Err(UserServiceError::EmailTaken)));
    }

    #[tokio::test]
    async fn update_with_an_invalid_role_is_rejected_and_changes_nothing() {
        let repo = Arc::new(FakeUserRepo::default());
        let service = UserService::new(repo.clone());
        let user = service
            .create("bob", "password123", "user", "bob@example.com", None)
            .await
            .unwrap();

        let result = service
            .update(user.id.unwrap(), Some("superuser"), None, None, None)
            .await;
        assert!(matches!(result, Err(UserServiceError::InvalidRole(_))));

        let unchanged = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.role, "user");
    }

    #[tokio::test]
    async fn update_with_an_invalid_email_is_rejected_and_changes_nothing() {
        let repo = Arc::new(FakeUserRepo::default());
        let service = UserService::new(repo.clone());
        let user = service
            .create("bob", "password123", "user", "bob@example.com", None)
            .await
            .unwrap();

        let result = service
            .update(user.id.unwrap(), None, None, Some("not-an-email"), None)
            .await;
        assert!(matches!(result, Err(UserServiceError::InvalidEmail(_))));

        let unchanged = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.email, "bob@example.com");
    }

    #[tokio::test]
    async fn update_password_only_rehashes_the_password_field() {
        let repo = Arc::new(FakeUserRepo::default());
        let service = UserService::new(repo.clone());
        let user = service
            .create(
                "carol",
                "old-password",
                "manager",
                "carol@example.com",
                None,
            )
            .await
            .unwrap();
        let original_hash = user.password_hash.clone();

        service
            .update(user.id.unwrap(), None, Some("new-password"), None, None)
            .await
            .unwrap();

        let updated = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.role, "manager"); // untouched
        assert_eq!(updated.email, "carol@example.com"); // untouched
        assert_ne!(updated.password_hash, original_hash);
        assert_ne!(updated.password_hash, "new-password"); // still hashed, not plaintext
    }

    #[tokio::test]
    async fn update_can_set_phone_without_touching_anything_else() {
        let repo = Arc::new(FakeUserRepo::default());
        let service = UserService::new(repo.clone());
        let user = service
            .create("dave", "password123", "user", "dave@example.com", None)
            .await
            .unwrap();

        service
            .update(user.id.unwrap(), None, None, None, Some("+1-555-0100"))
            .await
            .unwrap();

        let updated = repo
            .find_user_by_id(user.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.phone.as_deref(), Some("+1-555-0100"));
        assert_eq!(updated.email, "dave@example.com"); // untouched
    }
}
