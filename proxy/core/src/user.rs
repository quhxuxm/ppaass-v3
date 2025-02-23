use ppaass_common::error::CommonError;
use ppaass_common::user::repo::fs::FileSystemUserInfoRepository;
use ppaass_common::user::{UserInfo, UserInfoRepository};
use std::sync::Arc;
use tokio::sync::RwLock;
#[derive(Debug)]
pub struct ForwardProxyUserRepository {
    concrete_user_repo: FileSystemUserInfoRepository,
}
impl ForwardProxyUserRepository {
    pub fn new(concrete_user_repo: FileSystemUserInfoRepository) -> Self {
        Self { concrete_user_repo }
    }
}

#[async_trait::async_trait]
impl UserInfoRepository for ForwardProxyUserRepository {
    async fn get_user(&self, username: &str) -> Result<Option<Arc<RwLock<UserInfo>>>, CommonError> {
        self.concrete_user_repo.get_user(username).await
    }
    async fn get_single_user(
        &self,
    ) -> Result<Option<(String, Arc<RwLock<UserInfo>>)>, CommonError> {
        self.concrete_user_repo.get_single_user().await
    }
}
