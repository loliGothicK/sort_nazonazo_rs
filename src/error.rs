use std::io;
use std::path::Path;

quick_error! {
    #[derive(Debug)]
    pub enum BotError {
        Io(s: String, err: std::io::Error) {
            display("I/O error: {} => {}", err, s)
            context(path: &'a Path, err: io::Error) -> (path.to_string_lossy().to_string(), err)
        }
        Parse(s: &'static str, err: toml::ser::Error) {
            description(err.description())
            display("I/O error: {} => {}", err, s)
            context(s: &'static str, err: toml::ser::Error) -> (s, err)
        }
    }
}
