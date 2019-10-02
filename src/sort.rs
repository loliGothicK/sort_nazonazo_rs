use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

pub trait Sorted {
    fn sorted(&self) -> String;
}

impl Sorted for String {
    fn sorted(&self) -> Self {
        UnicodeSegmentation::graphemes(self.as_str(), true)
            .sorted()
            .collect::<String>()
    }
}
