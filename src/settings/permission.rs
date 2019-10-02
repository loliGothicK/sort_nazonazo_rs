use serde_derive::{Deserialize, Serialize};
#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct Channel {
    pub(crate) enabled: Vec<u64>,
}
