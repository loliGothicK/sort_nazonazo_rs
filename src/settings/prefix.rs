use indexmap::IndexMap;
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct Prefix {
    pub(crate) dynamic: IndexMap<String, String>,
}
