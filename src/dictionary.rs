use indexmap::IndexMap;
use itertools::Itertools;
use rand::distributions::{Distribution, Uniform};

use serde_derive::{Deserialize, Serialize};

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;

use std::{env, path::Path};

#[derive(Serialize, Deserialize, Debug)]
struct RawDictionary {
    pub questions: Vec<String>,
    pub full: Option<Vec<String>>,
}

struct CustomRawDictionary<F: Fn(&String) -> String> {
    pub questions: Vec<String>,
    pub full: Option<Vec<String>>,
    pub before: F,
}

impl RawDictionary {
    fn from_toml<Dic: AsRef<Path>>(dic: Dic) -> std::io::Result<RawDictionary> {
        let mut f = File::open(Path::new(&env::var("DIC_DIR").unwrap()).join(dic))?;
        let mut buffer = String::new();
        // config file open
        // read config.toml
        let _ = f.read_to_string(&mut buffer).unwrap();
        // parse toml
        let raw: RawDictionary =
            toml::from_slice(buffer.as_bytes()).expect("could not parse dictionary!");
        Ok(raw)
    }

    fn before<F: Fn(&String) -> String>(self, f: F) -> CustomRawDictionary<F> {
        CustomRawDictionary {
            questions: self.questions,
            full: self.full,
            before: f,
        }
    }
}

pub struct Dictionary {
    pub questions: IndexMap<String, String>,
    pub anagrams: BTreeMap<String, BTreeSet<String>>,
    pub full: Option<BTreeMap<String, BTreeSet<String>>>,
    dist: Uniform<usize>,
}

impl Dictionary {
    pub fn get<Rng: rand::Rng>(&self, engine: &mut Rng) -> Option<(&String, &String)> {
        self.questions.get_index(self.dist.sample(engine))
    }

    pub fn len(&self) -> usize {
        self.questions.len()
    }

    pub fn full_len(&self) -> Option<usize> {
        self.full.as_ref().map(|dic| dic.len())
    }

    pub fn get_anagrams(&self, sorted: &String) -> BTreeSet<String> {
        self.anagrams.get(sorted).unwrap().clone()
    }

    pub fn get_full_anagrams(&self, sorted: &String) -> Option<BTreeSet<String>> {
        self.full
            .as_ref()
            .map(|dic| dic.get(sorted).unwrap().clone())
    }
}

impl Into<Dictionary> for RawDictionary {
    fn into(self) -> Dictionary {
        let mut dictionary = IndexMap::new();
        for word in &self.questions {
            dictionary.insert(
                word.clone(),
                word.clone()
                    .chars()
                    .into_iter()
                    .sorted()
                    .collect::<String>(),
            );
        }

        let mut anagram_dictionary: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for word in &self.questions {
            let sorted = word
                .clone()
                .chars()
                .into_iter()
                .sorted()
                .collect::<String>();

            anagram_dictionary
                .entry(sorted.clone())
                .and_modify(|e| {
                    e.insert(word.clone());
                })
                .or_insert((|| {
                    let mut init = BTreeSet::new();
                    init.insert(word.clone());
                    init
                })());
        }

        let mut full_dictionary = None;
        if let Some(fully) = &self.full {
            let mut full_dic: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for word in fully {
                let sorted = word
                    .clone()
                    .chars()
                    .into_iter()
                    .sorted()
                    .collect::<String>();

                full_dic
                    .entry(sorted.clone())
                    .and_modify(|e| {
                        e.insert(word.clone());
                    })
                    .or_insert((|| {
                        let mut init = BTreeSet::new();
                        init.insert(word.clone());
                        init
                    })());
            }
            full_dictionary = Some(full_dic);
        }

        Dictionary {
            questions: dictionary.to_owned(),
            anagrams: anagram_dictionary.to_owned(),
            full: full_dictionary,
            dist: Uniform::new(0, dictionary.len()),
        }
    }
}

impl<F: Fn(&String) -> String> Into<Dictionary> for CustomRawDictionary<F> {
    fn into(self) -> Dictionary {
        let mut dictionary = IndexMap::new();
        for word in &self.questions {
            dictionary.insert(
                word.clone(),
                (&self.before)(word)
                    .chars()
                    .into_iter()
                    .sorted()
                    .collect::<String>(),
            );
        }

        let mut anagram_dictionary: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for word in &self.questions {
            let sorted = word
                .clone()
                .chars()
                .into_iter()
                .sorted()
                .collect::<String>();

            anagram_dictionary
                .entry(sorted.clone())
                .and_modify(|e| {
                    e.insert(word.clone());
                })
                .or_insert((|| {
                    let mut init = BTreeSet::new();
                    init.insert(word.clone());
                    init
                })());
        }

        let mut full_dictionary = None;
        if let Some(fully) = &self.full {
            let mut full_dic: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for word in fully {
                let sorted = word
                    .clone()
                    .chars()
                    .into_iter()
                    .sorted()
                    .collect::<String>();

                full_dic
                    .entry(sorted.clone())
                    .and_modify(|e| {
                        e.insert(word.clone());
                    })
                    .or_insert((|| {
                        let mut init = BTreeSet::new();
                        init.insert(word.clone());
                        init
                    })());
            }
            full_dictionary = Some(full_dic);
        }

        Dictionary {
            questions: dictionary.to_owned(),
            anagrams: anagram_dictionary.to_owned(),
            full: full_dictionary,
            dist: Uniform::new(0, dictionary.len()),
        }
    }
}

lazy_static! {
    pub static ref ENGLISH: Dictionary = {
        let dictionary: Dictionary = RawDictionary::from_toml("english.toml").unwrap().into();
        println!("ENGLISH is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref JAPANESE: Dictionary = {
        let dictionary: Dictionary = RawDictionary::from_toml("japanese.toml").unwrap().into();
        println!("JAPANESE is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref FRENCH: Dictionary = {
        let dictionary: Dictionary = RawDictionary::from_toml("french.toml").unwrap().into();
        println!("FRENCH is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref GERMAN: Dictionary = {
        let dictionary: Dictionary = RawDictionary::from_toml("german.toml")
            .unwrap()
            .before(|word| word.to_ascii_lowercase())
            .into();
        println!("GERMAN is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
    pub static ref ITALIAN: Dictionary = {
        let dictionary: Dictionary = RawDictionary::from_toml("italian.toml").unwrap().into();
        println!("ITALIAN is loaded: len = {}", dictionary.questions.len());
        dictionary
    };
}
