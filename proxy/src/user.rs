use ppaass_common::error::CommonError;
use ppaass_common::user::repo::fs::FileSystemUserInfoRepository;
use ppaass_common::user::{UserInfo, UserInfoRepository};
use std::sync::Arc;
#[derive(Debug)]
pub struct ForwardProxyUserRepository {
    concrete_user_repo: FileSystemUserInfoRepository,
}
impl ForwardProxyUserRepository {
    pub fn new(concrete_user_repo: FileSystemUserInfoRepository) -> Self {
        Self { concrete_user_repo }
    }
}
impl UserInfoRepository for ForwardProxyUserRepository {
    fn get_user(&self, username: &str) -> Result<Option<Arc<UserInfo>>, CommonError> {
        self.concrete_user_repo.get_user(username)
    }
    fn get_single_user(&self) -> Result<Option<(String, Arc<UserInfo>)>, CommonError> {
        self.concrete_user_repo.get_single_user()
    }
}
