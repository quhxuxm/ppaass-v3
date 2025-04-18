use crate::error::CommonError;
use crate::user::repo::fs::{
    FileSystemUserInfoRepository, FsAgentUserInfoContent, USER_INFO_ADDITION_INFO_PROXY_SERVERS,
};
use std::path::Path;
pub mod fs;

pub async fn create_fs_user_repository(
    user_dir: &Path,
    refresh_interval: u64,
) -> Result<FileSystemUserInfoRepository, CommonError> {
    FileSystemUserInfoRepository::new::<FsAgentUserInfoContent, _, _>(
        refresh_interval,
        user_dir,
        |user_info, content| async move {
            let mut user_info = user_info.write().await;
            user_info.add_additional_info(
                USER_INFO_ADDITION_INFO_PROXY_SERVERS,
                content.proxy_servers().to_owned(),
            );
        },
    )
    .await
}
