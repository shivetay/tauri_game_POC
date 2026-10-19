use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Good {
    Hides,
    Grain,
    Iron,
    Fish,
    Timber,
    Goods,
    Food,
    Ore,
    Herbs,
    Tools,
}

pub const ALL_GOODS: &[Good] = &[
    Good::Hides,
    Good::Grain,
    Good::Iron,
    Good::Fish,
    Good::Timber,
    Good::Goods,
    Good::Food,
    Good::Ore,
    Good::Herbs,
    Good::Tools,
];

impl Good {
    pub fn label(self) -> &'static str {
        match self {
            Good::Hides => "skóry",
            Good::Grain => "zboże",
            Good::Iron => "żelazo",
            Good::Fish => "ryby",
            Good::Timber => "drewno",
            Good::Goods => "towary",
            Good::Food => "żywność",
            Good::Ore => "ruda",
            Good::Herbs => "zioła",
            Good::Tools => "narzędzia",
        }
    }
}
