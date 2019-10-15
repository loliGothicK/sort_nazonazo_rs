use indexmap::IndexSet;
use rand::distributions::{Distribution, Uniform};
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

use std::{env, path::Path};

#[derive(Debug)]
pub struct Dictionary {
    questions: IndexSet<String>,
    full: Option<IndexSet<String>>,
    dist: Uniform<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
struct RawDictionary {
    questions: Vec<String>,
    full: Option<Vec<String>>,
}

impl Dictionary {
    pub fn get<Rng: rand::Rng>(&self, engine: &mut Rng) -> &String {
        self.questions.get_index(self.dist.sample(engine)).unwrap()
    }

    pub fn len(&self) -> usize {
        self.questions.len()
    }

    pub fn full_len(&self) -> Option<usize> {
        self.full.as_ref().map(|dic| dic.len())
    }

    pub fn contains(&self, word: &str) -> bool {
        self.questions.contains(word)
    }

    pub fn contains_ex(&self, word: &str) -> bool {
        self.full
            .as_ref()
            .map(|x| x.contains(word))
            .unwrap_or(false)
    }

    pub fn from_toml<S: AsRef<Path>>(file: S) -> std::io::Result<Dictionary> {
        let mut f = File::open(Path::new(&env::var("DIC_DIR").unwrap()).join(file))?;
        let mut buffer = String::new();
        // config file open
        // read config.toml
        let _ = f.read_to_string(&mut buffer).unwrap();
        // parse toml
        let raw: RawDictionary =
            toml::from_slice(buffer.as_bytes()).expect("could not parse dictionary!");
        let mut questions = IndexSet::new();
        for word in raw.questions {
            questions.insert(word.to_lowercase());
        }
        let full = if let Some(full) = raw.full {
            let mut full_dic = IndexSet::new();
            for word in full {
                full_dic.insert(word.to_lowercase());
            }
            Some(full_dic)
        } else {
            None
        };
        let dist = Uniform::new(0, questions.len());
        Ok(Dictionary {
            questions,
            full,
            dist,
        })
    }
}

lazy_static! {
    pub static ref ENGLISH: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("english.toml").unwrap();
        println!("ENGLISH is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref JAPANESE: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("japanese.toml").unwrap();
        println!("JAPANESE is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref FRENCH: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("french.toml").unwrap();
        println!("FRENCH is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref GERMAN: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("german.toml").unwrap();
        println!("GERMAN is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref ITALIAN: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("italian.toml").unwrap();
        println!("ITALIAN is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref RUSSIAN: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("russian.toml").unwrap();
        println!("RUSSIAN is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref ESPERANTO: Dictionary = {
        let dictionary: Dictionary = Dictionary::from_toml("esperanto.toml").unwrap();
        println!("ESPERANTO is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
}
