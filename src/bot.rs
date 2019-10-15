use super::dictionary::*;
use super::sort::Sorted;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use rand::distributions::{Distribution, Uniform};
use serenity::client::Context;
use serenity::model::channel::Message;

use std::ops::AddAssign;
use std::sync::{Arc, Mutex};
use std::time::Instant;

custom_derive! {
    #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, NextVariant, PrevVariant)]
    pub enum Lang {
        En,
        Ja,
        Fr,
        De,
        It,
        Ru,
        Eo,
    }
}

impl Lang {
    pub fn as_symbol(self) -> String {
        match self {
            Lang::En => "英単語".to_string(),
            Lang::Ja => "単語".to_string(),
            Lang::Fr => "仏単語".to_string(),
            Lang::De => "独単語".to_string(),
            Lang::It => "伊単語".to_string(),
            Lang::Ru => "露単語".to_string(),
            Lang::Eo => "エスペラント単語".to_string(),
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
            eo if eo == "eo" => Lang::Eo,
            _ => panic!("unexpected language token!"),
        }
    }
}

pub fn get_dictionary(lang: Lang) -> &'static Dictionary {
    match lang {
        Lang::En => &*ENGLISH,
        Lang::Ja => &*JAPANESE,
        Lang::Fr => &*FRENCH,
        Lang::De => &*GERMAN,
        Lang::It => &*ITALIAN,
        Lang::Ru => &*RUSSIAN,
        Lang::Eo => &*ESPERANTO,
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
        eo if eo == "eo" => &*ESPERANTO,
        _ => panic!("unexpected language token!"),
    }
}

#[derive(Debug)]
pub enum Status {
    StandingBy,
    Holding(String, Lang, Instant),
    Contesting(String, Lang, (u32, u32), Instant),
}

pub enum CheckResult<'a> {
    Assumed(&'a str),
    Anagram(&'a str),
    Full(&'a str),
    WA,
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

    pub fn get_dictionary(&self) -> Result<&Dictionary, ()> {
        match self {
            Status::StandingBy => Err(()),
            Status::Contesting(_, lang, ..) | Status::Holding(_, lang, ..) => {
                Ok(get_dictionary(*lang))
            }
        }
    }

    pub fn is_correct_answer(&self, got: &str) -> bool {
        match self {
            Status::StandingBy => false,
            Status::Contesting(ans, ..) | Status::Holding(ans, ..) => ans == &got.to_lowercase(),
        }
    }

    pub fn is_anagram(&self, got: &str) -> bool {
        match self {
            Status::StandingBy => false,
            Status::Contesting(ans, ..) | Status::Holding(ans, ..) => {
                ans.sorted() == got.to_lowercase().sorted()
                    && self.get_dictionary().unwrap().contains(&got.to_lowercase())
            }
        }
    }

    pub fn is_anagram_by_full(&self, got: &str) -> bool {
        match self {
            Status::StandingBy => false,
            _ => {
                self.ans().unwrap().sorted() == got.to_lowercase().sorted()
                    && self
                        .get_dictionary()
                        .unwrap()
                        .contains_ex(&got.to_lowercase())
            }
        }
    }

    pub fn answer_check<'a>(&self, msg: &'a str) -> CheckResult<'a> {
        match self {
            _ if self.is_correct_answer(msg) => CheckResult::Assumed(msg),
            _ if self.is_anagram(msg) => CheckResult::Anagram(msg),
            _ if self.is_anagram_by_full(msg) => CheckResult::Full(msg),
            _ => CheckResult::WA,
        }
    }

    pub fn is_contest_end(&self) -> bool {
        match self {
            Status::Contesting(_, _, (count, num), ..) => *count == *num,
            _ => false,
        }
    }

    pub fn get_contest_num(&self) -> Option<(&u32, &u32)> {
        match self {
            Status::Contesting(_, _, (count, num), ..) => Some((count, num)),
            _ => None,
        }
    }

    pub fn contest_continue(&mut self, ctx: &mut Context, msg: &Message) {
        let (dic, lang) = CONTEST_LIBRARY
            .lock()
            .unwrap()
            .select(&mut rand::thread_rng());
        let ans = dic.get(&mut rand::thread_rng());
        let sorted = ans.sorted();
        println!("called contest_continue: [{}, {}]", ans, sorted);
        let (count, num) = self.get_contest_num().unwrap();
        msg.channel_id
            .say(
                &ctx,
                format!(
                    "問 {current} ({current}/{number})\nソートなぞなぞ ソート前の {symbol} な〜んだ？\n{prob}",
                    number = num,
                    current = *count + 1,
                    prob = sorted,
                    symbol = lang.as_symbol(),
                ),
            )
            .expect("fail to post");
        *self = Status::Contesting(ans.to_string(), lang, (*count + 1, *num), Instant::now());
    }

    pub fn elapsed(&self) -> Option<f32> {
        match self {
            Status::Holding(_, _, instant) => Some(instant.elapsed().as_secs_f32()),
            Status::Contesting(_, _, _, instant) => Some(instant.elapsed().as_secs_f32()),
            _ => None,
        }
    }
}

pub struct DictionarySelector {
    engine: Result<Lang, Uniform<usize>>,
    set: IndexSet<Lang>,
}

impl Default for DictionarySelector {
    fn default() -> Self {
        Self::new()
    }
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
        (get_dictionary(lang), lang)
    }
}

#[derive(Default, Debug)]
pub struct ContestData {
    pub time: Vec<f32>,
}

impl ContestData {
    pub fn as_string(&self) -> String {
        format!(
            "{} AC, average speed = {:.3} sec",
            self.time.len(),
            self.time.iter().sum::<f32>() / self.time.len() as f32
        )
    }

    pub fn key(&self) -> (i32, u32) {
        (
            -(self.time.len() as i32),
            (self.time.iter().map(|x| x * 1000.0).sum::<f32>() / self.time.len() as f32) as u32,
        )
    }
}

impl AddAssign<f32> for ContestData {
    fn add_assign(&mut self, rhs: f32) {
        self.time.push(rhs);
    }
}

pub fn aggregates(contest_result: &IndexMap<String, ContestData>) -> String {
    use ordinal::Ordinal;
    contest_result
        .iter()
        .sorted_by_key(|(_, data)| data.key())
        .enumerate()
        .map(|(index, (name, data))| {
            format!(
                "{}: {}, {}\n",
                Ordinal(index + 1).to_string(),
                name,
                data.as_string()
            )
        })
        .collect::<String>()
}

lazy_static! {
    pub static ref QUIZ: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
    pub static ref CONTEST_REUSLT: Arc<Mutex<IndexMap<String, ContestData>>> =
        Arc::new(Mutex::new(IndexMap::new()));
    pub static ref CONTEST_LIBRARY: Arc<Mutex<DictionarySelector>> =
        Arc::new(Mutex::new(DictionarySelector::new()));
}
