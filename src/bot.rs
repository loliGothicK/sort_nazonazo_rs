use super::dictionary::*;
use indexmap::IndexSet;
use rand::distributions::{Distribution, Uniform};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;
use std::sync::{Arc, Mutex};

custom_derive! {
    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, NextVariant, PrevVariant)]
    pub enum Lang {
        En,
        Ja,
        Fr,
        De,
        It,
        Ru,
    }
}

impl Lang {
    pub fn as_symbol(&self) -> String {
        match self {
            Lang::En => "英単語".to_string(),
            Lang::Ja => "単語".to_string(),
            Lang::Fr => "仏単語".to_string(),
            Lang::De => "独単語".to_string(),
            Lang::It => "伊単語".to_string(),
            Lang::Ru => "露単語".to_string(),
        }
    }
}

impl<S: Into<String>> From<S> for Lang {
    fn from(s: S) -> Self {
        let lang: String = s.into();
        match lang {
            en if en == "en" => Lang::En,
            ja if ja == "ja" => Lang::Ja,
            fr if fr == "fr" => Lang::Fr,
            de if de == "de" => Lang::De,
            it if it == "it" => Lang::It,
            ru if ru == "ru" => Lang::Ru,
            _ => panic!("unexpected language token!"),
        }
    }
}

pub enum Status {
    StandingBy,
    Holding(String, String, BTreeSet<String>, Option<BTreeSet<String>>),
    Contesting(
        String,
        String,
        (u32, u32),
        BTreeSet<String>,
        Option<BTreeSet<String>>,
    ),
}

impl Status {
    pub fn is_standing_by(&self) -> bool {
        match self {
            Status::StandingBy => true,
            _ => false,
        }
    }
    pub fn is_holding(&self) -> bool {
        match self {
            Status::Holding(..) => true,
            _ => false,
        }
    }
    pub fn is_contesting(&self) -> bool {
        match self {
            Status::Contesting(..) => true,
            _ => false,
        }
    }

    pub fn ans(&self) -> std::result::Result<&String, ()> {
        match self {
            Status::StandingBy => Err(()),
            Status::Holding(ans, ..) | Status::Contesting(ans, ..) => Ok(ans),
        }
    }
}

pub fn select_dictionary(lang: Lang) -> &'static Dictionary {
    match lang {
        Lang::En => &*ENGLISH,
        Lang::Ja => &*JAPANESE,
        Lang::Fr => &*FRENCH,
        Lang::De => &*GERMAN,
        Lang::It => &*ITALIAN,
        Lang::Ru => &*RUSSIAN,
    }
}

pub fn select_dictionary_from_str<S: Into<String>>(lang: S) -> &'static Dictionary {
    let lang_string: String = lang.into();
    match lang_string {
        en if en == "en" => &*ENGLISH,
        ja if ja == "ja" => &*JAPANESE,
        fr if fr == "fr" => &*FRENCH,
        de if de == "de" => &*GERMAN,
        it if it == "it" => &*ITALIAN,
        ru if ru == "ru" => &*RUSSIAN,
        _ => panic!("unexpected language token!"),
    }
}

pub struct DictionarySelector {
    engine: Result<Lang, Uniform<usize>>,
    set: IndexSet<Lang>,
}

impl DictionarySelector {
    pub fn new() -> DictionarySelector {
        DictionarySelector {
            engine: Ok(Lang::En),
            set: Default::default(),
        }
    }
    pub fn set<S: Into<String>>(&mut self, languages: Vec<S>) {
        if languages.len() == 1 {
            self.engine = Ok(Lang::from(languages.into_iter().next().unwrap()));
        } else {
            self.engine = Err(Uniform::new(0, languages.len()));
            for lang in languages {
                self.set.insert(Lang::from(lang));
            }
        }
    }
    pub fn select<Engine: rand::Rng>(&self, rng: &mut Engine) -> (&'static Dictionary, Lang) {
        let lang = *self
            .engine
            .as_ref()
            .unwrap_or_else(|uniform| self.set.get_index(uniform.sample(rng)).unwrap());
        (select_dictionary(lang.clone()), lang)
    }
}

lazy_static! {
    pub static ref QUIZ: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
    pub static ref CONTEST_REUSLT: Arc<Mutex<BTreeMap<String, u32>>> = Arc::new(Mutex::new(BTreeMap::new()));
    pub static ref CONTEST_LIBRARY: Arc<Mutex<DictionarySelector>> = Arc::new(Mutex::new(DictionarySelector::new()));
}
