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
    ("shared", "Co víš s ostatními", "What you share with others"),
    ("faction", "Tvá frakce", "Your faction"),
    ("pack_title", "Balicí seznam", "Packing list"),
    ("pack_before", "Před hrou", "Before the game"),
    ("pack_during", "Během hry", "During the game"),
    ("pack_by", "vydává", "handed out by"),
    ("pack_handout", "kartička", "handout"),
    (
        "pack_empty",
        "Hra nemá žádné předměty ani kartičky.",
        "The game has no items or handouts.",
    ),
    ("handouts_title", "Kartičky pro hráče", "Handouts"),
    (
        "handouts_empty",
        "Žádná informace ani předmět nemá kartičku (handout).",
        "No item or info has a handout.",
    ),
    ("pack_unassigned", "Nikdo to nemá", "Nobody holds it"),
    ("pack_members", "členové", "members"),
    ("pack_for", "pro", "for"),
    ("pack_if", "při volbě", "if"),
    ("q_open", "„", "“"),
    ("q_close", "“", "”"),
    ("faction_members", "Další členové", "Other members"),
    ("faction_goals", "Společné cíle", "Shared goals"),
    ("faction_secrets", "Společná tajemství", "Shared secrets"),
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
    // Organizer sheets.
    ("room", "místnost", "room"),
    ("faction_kind", "frakce", "faction"),
    ("org_title", "Rozpis", "Schedule"),
    ("org_load", "Vytížení", "Load"),
    ("org_total", "akcí", "actions"),
    ("org_decisions", "rozhodnutí", "decisions"),
    ("org_peak", "nejvíc", "busiest"),
    ("org_prepare", "Připrav si", "Have ready"),
    ("org_schedule", "Průběh", "Running order"),
    ("org_with", "S kým", "With"),
    ("org_from", "od", "from"),
    ("org_needs", "Potřebuje", "Needs"),
    ("org_uses", "Spotřebuje", "Uses up"),
    ("org_gives", "Dává", "Gives"),
    ("org_brief", "Hráči čtou", "Players read"),
    ("org_decision", "Rozhodnutí", "Decision"),
    ("org_none", "Nic na starosti.", "Nothing to run."),
    ("org_no_segment", "Mimo segmenty", "Outside segments"),
    ("org_info", "informace", "info"),
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
