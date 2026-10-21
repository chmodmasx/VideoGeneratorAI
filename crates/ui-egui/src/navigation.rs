#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Create,
    Compose,
    Character,
    Avatar,
    Sound,
    Enhance,
    Train,
    Library,
    Queue,
    Models,
    Settings,
}

impl Section {
    pub const ALL: [Self; 11] = [
        Self::Create,
        Self::Compose,
        Self::Character,
        Self::Avatar,
        Self::Sound,
        Self::Enhance,
        Self::Train,
        Self::Library,
        Self::Queue,
        Self::Models,
        Self::Settings,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Compose => "Compose",
            Self::Character => "Character",
            Self::Avatar => "Avatar",
            Self::Sound => "Sound",
            Self::Enhance => "Enhance",
            Self::Train => "Train",
            Self::Library => "Library",
            Self::Queue => "Queue",
            Self::Models => "Models",
            Self::Settings => "Settings",
        }
    }
}
