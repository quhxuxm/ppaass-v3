use accessory::Accessors;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Debug, Accessors)]
pub struct ProxyToolConfig {
    #[access(get(ty=&std::path::Path))]
    pub user_dir: PathBuf,
}
