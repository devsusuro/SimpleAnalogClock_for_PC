#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Language {
    #[default]
    Korean,
    English,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::Korean, Self::English];

    pub fn label(self) -> &'static str {
        self.text("한국어", "English")
    }

    pub fn text(self, korean: &'static str, english: &'static str) -> &'static str {
        match self {
            Self::Korean => korean,
            Self::English => english,
        }
    }

    pub fn ini_value(self) -> &'static str {
        self.text("ko", "en")
    }

    pub fn from_ini(value: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|language| language.ini_value().eq_ignore_ascii_case(value))
            .unwrap_or_default()
    }
}
