//! Portfolios: separate wallets in one database (e.g. your own and the ones
//! you manage for relatives), and who may open each one.
//!
//! This service is the single place that decides whether a user can see a
//! portfolio. Every `/api/portfolios/{id}/...` route goes through
//! `access()` before touching any data, and a user who isn't a member gets
//! `NotFound` — the same answer as for a portfolio that doesn't exist, so
//! the API never reveals that someone else's portfolio is there.

use crate::domain::models::{PortfolioAccess, PortfolioId, PortfolioSummary};
use crate::domain::repository::{PortfolioDirectoryRepo, RepoError, UniqueViolation};
use crate::usecases::validation::check_not_blank;
use std::fmt;
use std::sync::Arc;

/// Long enough for "Roberto — Previdência", short enough for the switcher.
const MAX_NAME_CHARS: usize = 60;

#[derive(Debug)]
pub enum PortfolioError {
    /// No such portfolio, or the user isn't a member (deliberately the same).
    NotFound,
    /// A member with `view` access tried to change something.
    ReadOnly,
    Invalid(String),
    NameTaken,
    Repo(RepoError),
}

impl fmt::Display for PortfolioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortfolioError::NotFound => write!(f, "portfolio not found"),
            PortfolioError::ReadOnly => write!(f, "you have read-only access to this portfolio"),
            PortfolioError::Invalid(message) => write!(f, "{message}"),
            PortfolioError::NameTaken => write!(f, "a portfolio with this name already exists"),
            PortfolioError::Repo(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PortfolioError {}

impl From<RepoError> for PortfolioError {
    fn from(e: RepoError) -> Self {
        if e.downcast_ref::<UniqueViolation>().is_some() {
            PortfolioError::NameTaken
        } else {
            PortfolioError::Repo(e)
        }
    }
}

pub struct PortfolioService {
    directory: Arc<dyn PortfolioDirectoryRepo>,
}

impl PortfolioService {
    pub fn new(directory: Arc<dyn PortfolioDirectoryRepo>) -> Self {
        Self { directory }
    }

    /// The portfolios `user_id` can open, by name.
    pub async fn list_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<PortfolioSummary>, PortfolioError> {
        Ok(self.directory.list_portfolios_for_user(user_id).await?)
    }

    /// `user_id`'s access to `portfolio`; `NotFound` if not a member.
    pub async fn access(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
    ) -> Result<PortfolioAccess, PortfolioError> {
        self.directory
            .find_access(portfolio, user_id)
            .await?
            .ok_or(PortfolioError::NotFound)
    }

    /// Creates a portfolio owned (`manage`) by `owner_id`. Its BARCA targets
    /// start as a copy of `copy_barca_from`'s, which the owner must be able
    /// to open — otherwise someone could copy a portfolio they can't see.
    pub async fn create(
        &self,
        name: &str,
        owner_id: i64,
        copy_barca_from: Option<PortfolioId>,
    ) -> Result<PortfolioSummary, PortfolioError> {
        let name = check_name(name)?;
        if let Some(source) = copy_barca_from {
            self.access(source, owner_id).await?;
        }
        let id = self
            .directory
            .create_portfolio(&name, owner_id, copy_barca_from)
            .await?;
        tracing::info!(portfolio_id = %id, owner_id, ?copy_barca_from, "Created portfolio");
        Ok(PortfolioSummary {
            id,
            name,
            access: PortfolioAccess::Manage,
        })
    }

    /// Renames a portfolio; needs `manage` access.
    pub async fn rename(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
        name: &str,
    ) -> Result<(), PortfolioError> {
        let name = check_name(name)?;
        if self.access(portfolio, user_id).await? != PortfolioAccess::Manage {
            return Err(PortfolioError::ReadOnly);
        }
        if !self.directory.rename_portfolio(portfolio, &name).await? {
            return Err(PortfolioError::NotFound);
        }
        tracing::info!(portfolio_id = %portfolio, user_id, "Renamed portfolio");
        Ok(())
    }

    /// Makes `user_id` a member of `portfolio` (first-run seeding).
    pub async fn add_member(
        &self,
        portfolio: PortfolioId,
        user_id: i64,
        access: PortfolioAccess,
    ) -> Result<(), PortfolioError> {
        Ok(self
            .directory
            .set_member(portfolio, user_id, access)
            .await?)
    }
}

fn check_name(name: &str) -> Result<String, PortfolioError> {
    let name = name.trim();
    check_not_blank("name", name).map_err(PortfolioError::Invalid)?;
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(PortfolioError::Invalid(format!(
            "name must be at most {MAX_NAME_CHARS} characters"
        )));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository::{BarcaTargetRepo, NewUser, UserRepo};
    use crate::infra::sqlite::repo::test_support::barca;
    use crate::infra::sqlite::{SqliteRepo, test_repo};

    /// A migrated database (so portfolio 1 exists) with users 1 and 2, and
    /// user 1 a `manage` member of portfolio 1.
    async fn setup() -> (Arc<SqliteRepo>, PortfolioService) {
        let repo = test_repo().await;
        for name in ["owner", "other"] {
            repo.create_user(NewUser {
                username: name,
                password_hash: "hash",
                role: "manager",
                email: &format!("{name}@example.com"),
                phone: None,
            })
            .await
            .unwrap();
        }
        let service = PortfolioService::new(repo.clone());
        service
            .add_member(PortfolioId::FIRST, 1, PortfolioAccess::Manage)
            .await
            .unwrap();
        (repo, service)
    }

    #[tokio::test]
    async fn a_non_member_gets_not_found_like_a_missing_portfolio() {
        let (_, service) = setup().await;
        assert!(matches!(
            service.access(PortfolioId::FIRST, 2).await,
            Err(PortfolioError::NotFound)
        ));
        assert!(matches!(
            service.access(PortfolioId(999), 1).await,
            Err(PortfolioError::NotFound)
        ));
        assert_eq!(
            service.access(PortfolioId::FIRST, 1).await.unwrap(),
            PortfolioAccess::Manage
        );
    }

    #[tokio::test]
    async fn create_makes_the_owner_a_manager_and_lists_it_only_for_them() {
        let (_, service) = setup().await;
        let created = service.create("  Spouse  ", 1, None).await.unwrap();

        assert_eq!(created.name, "Spouse", "name is trimmed");
        let names: Vec<_> = service
            .list_for_user(1)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["My portfolio", "Spouse"]);
        assert!(service.list_for_user(2).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_copies_barca_targets_without_sharing_them() {
        let (repo, service) = setup().await;
        repo.replace_barca_targets(
            PortfolioId::FIRST,
            "BullMarket",
            &[barca("Base", 60.0), barca("Altcoins", 40.0)],
            None,
        )
        .await
        .unwrap();

        let copy = service
            .create("Sister", 1, Some(PortfolioId::FIRST))
            .await
            .unwrap();
        repo.replace_barca_targets(copy.id, "BullMarket", &[barca("Base", 100.0)], None)
            .await
            .unwrap();

        let original = repo
            .fetch_barca_targets(PortfolioId::FIRST, "BullMarket")
            .await
            .unwrap();
        assert_eq!(
            original.len(),
            2,
            "editing the copy left the original alone"
        );
    }

    #[tokio::test]
    async fn create_refuses_to_copy_from_a_portfolio_the_owner_cannot_open() {
        let (_, service) = setup().await;
        let err = service
            .create("Sneaky", 2, Some(PortfolioId::FIRST))
            .await
            .unwrap_err();
        assert!(matches!(err, PortfolioError::NotFound), "{err:?}");
        assert!(service.list_for_user(2).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn names_must_be_present_short_and_unique() {
        let (_, service) = setup().await;
        assert!(matches!(
            service.create("   ", 1, None).await,
            Err(PortfolioError::Invalid(_))
        ));
        assert!(matches!(
            service.create(&"x".repeat(61), 1, None).await,
            Err(PortfolioError::Invalid(_))
        ));
        assert!(matches!(
            service.create("My portfolio", 1, None).await,
            Err(PortfolioError::NameTaken)
        ));
    }

    #[tokio::test]
    async fn rename_needs_manage_access() {
        let (_, service) = setup().await;
        service
            .add_member(PortfolioId::FIRST, 2, PortfolioAccess::View)
            .await
            .unwrap();

        assert!(matches!(
            service.rename(PortfolioId::FIRST, 2, "Mine now").await,
            Err(PortfolioError::ReadOnly)
        ));
        service
            .rename(PortfolioId::FIRST, 1, "Roberto")
            .await
            .unwrap();
        assert_eq!(service.list_for_user(1).await.unwrap()[0].name, "Roberto");
    }
}
