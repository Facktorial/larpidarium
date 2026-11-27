//! Fixed texts printed on sheets, in each supported language.

use std::collections::BTreeMap;

use larp_model::model::Language;

const LABELS: &[(&str, &str, &str)] = &[
    // key, Czech, English
    ("back", "Zpět na graf", "Back to the graph"),
    ("player", "hráčská postava", "player character"),
    ("organizer", "organizátor", "organizer"),
    ("npc", "cizí postava", "non-player character"),
    ("part1", "Kdo jsi", "Who you are"),
    ("part2", "Tvůj osud", "Your destiny"),
    ("appearance", "Vzhled", "Appearance"),
    (
        "whereabouts",
        "Kde jsi na začátku hry",
        "Where you are when the game starts",
    ),
    ("backstory", "Tvůj příběh", "Your story"),
    ("goals", "Co chceš", "What you want"),
    ("secrets", "Co skrýváš", "What you hide"),
    ("relations", "Koho znáš", "People you know"),
    ("carries", "Co máš u sebe", "What you carry"),
    (
        "destiny_intro",
        "Takto se tvá cesta proplétá s hrou. Pořadí je přibližné; jak a kdy přesně, záleží na hře.",
        "This is how your path weaves through the game. The order is approximate; how and when exactly depends on play.",
    ),
    (
        "no_destiny",
        "Tvá postava zatím nemá v ději žádnou roli.",
        "Your character has no part in the plot yet.",
    ),
    ("src_carry", "máš u sebe", "you carry it"),
    ("src_earlier", "získáš dříve", "you get it earlier"),
    (
        "src_receive",
        "dostaneš nebo se dozvíš",
        "you will receive or learn it",
    ),
    ("needs", "Budeš potřebovat", "You will need"),
    ("uses", "Přijdeš o", "You will give up"),
    ("gives", "Výsledek", "Outcome"),
    ("choice", "Tvé rozhodnutí", "Your decision"),
    ("mini_own", "tvé činy", "your actions"),
    ("mini_other", "jiní", "others"),
    ("played_by", "Hraje", "Played by"),
    (
        "whereabouts_enter",
        "Kde jsi, když vstupuješ do příběhu",
        "Where you are when you enter the story",
    ),
    // Section headings for `[person.sheet]` keys in Part I extras and phases (`sheet_<key>`).
    (
        "sheet_echo",
        "Ozvěna minulého života",
        "Echo of a past life",
    ),
    ("sheet_appearance", "Vzhled", "Appearance"),
    ("sheet_whereabouts", "Kde jsi", "Where you are"),
    ("sheet_backstory", "Tvůj příběh", "Your story"),
    ("enters", "Vstupuješ do příběhu", "You enter the story"),
    ("dies", "Umíráš", "You die"),
    ("from_start", "od začátku", "from the start"),
    ("to_end", "dožiješ konce", "you live to the end"),
    ("outside_cycle", "mimo cyklus", "outside the cycle"),
    ("part3", "Jak se měníš", "How you change"),
    ("from_segment", "Od", "From"),
];

/// Labels for `language`, keyed by the first column of the table above.
#[must_use]
pub fn labels(language: Language) -> BTreeMap<&'static str, &'static str> {
    LABELS
        .iter()
        .map(|(key, cs, en)| {
            (
                *key,
                match language {
                    Language::Cs => *cs,
                    Language::En => *en,
                },
            )
        })
        .collect()
}
