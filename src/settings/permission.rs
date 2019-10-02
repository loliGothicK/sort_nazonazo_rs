use toml;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Default, Debug, Serialize, Deserialize)]
struct Channel {
    enabled: Vec<u64>
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    channel: Channel
}

pub fn init_config<ConfigPath: AsRef<Path>>(path: ConfigPath) -> std::io::Result<Config> {
    File::open("foo.txt").map_or_else(|_| {
        let mut f = File::create(path)?;
        let buffer = toml::to_string(&Config::default()).unwrap();
        f.write_all(buffer.as_bytes())?;
        f.sync_all()?;
        Ok(Config::default())
    },
    |mut file| {
        let mut buffer = String::new();
        let _ = file.read_to_string(&mut buffer).unwrap();
        let conf: Config = toml::from_slice(buffer.as_bytes())?;
        Ok(conf)
    })
}
