pub mod permission;

use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use toml;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) channel: permission::Channel,
}

lazy_static! {
    pub(crate) static ref SETTINGS: Arc<Mutex<Config>> = Arc::new(Mutex::new(init_config("/tmp/settings/settings.toml").unwrap()));
}

pub(crate) fn init_config<ConfigPath: AsRef<Path>>(path: ConfigPath) -> std::io::Result<Config> {
    File::open(&path).map_or_else(
        |_| {
            let mut f = File::create(&path)?;
            let buffer = toml::to_string(&Config::default()).unwrap();
            f.write_all(buffer.as_bytes())?;
            f.sync_all()?;
            Ok(Config::default())
        },
        |mut file| {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            let conf: Config = toml::from_slice(buffer.as_bytes())?;
            Ok(conf)
        },
    )
}
