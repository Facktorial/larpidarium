//! Snapshots of the four built-in sheets for one small, fixed game.
//!
//! Review changes with `cargo insta review`; accept them only when the new output is intended.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{Destiny, Schedule};
use larp_model::{Game, Id, SourceFile, load};
use larp_render::{
    organizer_sheet_html, organizer_sheets_tex, player_sheet_html, player_sheets_tex,
};

/// Two players in a two-step cycle, an organizer and a room. Texts are Czech and contain
/// characters that HTML and LaTeX must escape.
fn game() -> Game {
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"Řád & Kříž\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\"]\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"noc\"\nname = \"Noc_1\"\n\n[[segment]]\nid = \"rano\"\nname = \"Ráno 50 %\"\n",
        ),
        (
            "cycles.toml",
            "[[cycle]]\nid = \"anin\"\nname = \"Anin cyklus\"\nhero = \"ann\"\nfrom = \"noc\"\nto = \"rano\"\n",
        ),
        (
            "players.toml",
            "[[player]]\nid = \"p1\"\nname = \"Petra Ůlehlová\"\n",
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Anna <Nováková> #1\"\nrole = \"player\"\ncolor = \"#c0392b\"\nplayed_by = \"p1\"\ndies = 2\ngoals = [\"Přežít {noc}\"]\nsecrets = [\"Ukradla $5 & lhala\"]\n\n[person.sheet]\nappearance = \"Černý plášť\"\nwhereabouts = \"V kapli\"\nbackstory = \"Kuchařka.\\n\\nDruhý odstavec ~ ^\"\n\"vůně\" = \"Česnek\"\n\n[[person.relation]]\nwith = \"bob\"\ntext = \"Bratr \\\"Bob\\\" [starší]\"\n\n[[person.phase]]\nstep = 2\ngoals = [\"Utéct\"]\n[person.phase.sheet]\nwhereabouts = \"U brány\"\n",
        ),
        (
            "people/bob.toml",
            "[person]\nname = \"Bob Čech\"\nrole = \"player\"\ngoals = [\"Najít klíč\"]\n",
        ),
        (
            "people/org.toml",
            "[person]\nname = \"Organizátor_ka\"\nrole = \"organizer\"\ncolor = \"#2e86c1\"\n",
        ),
        ("rooms.toml", "[[room]]\nid = \"kaple\"\nname = \"Kaple\"\n"),
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Klíč {starý}\"\ntext = \"Rezavý <těžký>\"\nowners = [\"ann\"]\nstarting = true\n\n[[item]]\nid = \"svice\"\nname = \"Svíce\"\ntext = \"Hoří\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"konec\"\nname = \"Konec\"\nending = true\n",
        ),
        (
            "actions/a-hledani.toml",
            "[action]\ntitle = \"Hledání v kapli\"\ncycle = \"anin\"\nstep = 1\nowners = [\"ann\", \"org\", \"kaple\"]\nneeds = [\"klic\"]\ngives = [\"svice\"]\ntext = \"Org: schovej svíci & čekej.\"\nbrief = \"Hledáš světlo.\"\n",
        ),
        (
            "actions/b-volba.toml",
            "[decision]\ntitle = \"Velmi dlouhé rozhodnutí o osudu\"\ncycle = \"anin\"\nstep = 2\nowners = [\"ann\", \"bob\"]\nbrief = \"Co uděláš?\"\n\n[[decision.option]]\nlabel = \"Zůstat\"\nuses = [\"svice\"]\ngives = [\"konec\"]\n\n[[decision.option]]\nlabel = \"Odejít_hned\"\ngives = [\"konec\"]\n",
        ),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: (*t).to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:#?}", checked.diagnostics);
    checked.game.unwrap()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

fn destinies(g: &Game, people: &[&Id]) -> Vec<Destiny> {
    let graph = larp_graph::build(g, &[], None);
    people
        .iter()
        .map(|p| larp_graph::destiny(g, &graph, p))
        .collect()
}

fn schedules(g: &Game, owners: &[&str]) -> Vec<Schedule> {
    owners
        .iter()
        .map(|o| larp_graph::schedule(g, &id(o)))
        .collect()
}

#[test]
fn player_sheet_html_matches_snapshot() {
    let g = game();
    let ann = id("ann");
    let d = destinies(&g, &[&ann]);
    insta::assert_snapshot!(player_sheet_html(&g, &ann, &d[0], None).unwrap());
}

#[test]
fn player_sheets_tex_matches_snapshot() {
    let g = game();
    let (ann, bob) = (id("ann"), id("bob"));
    let d = destinies(&g, &[&ann, &bob]);
    let people = [(&ann, &d[0]), (&bob, &d[1])];
    insta::assert_snapshot!(player_sheets_tex(&g, &people, None).unwrap());
}

#[test]
fn organizer_sheet_html_matches_snapshot() {
    let g = game();
    let s = schedules(&g, &["org"]);
    insta::assert_snapshot!(organizer_sheet_html(&g, &s[0], None).unwrap());
}

#[test]
fn organizer_sheets_tex_matches_snapshot() {
    let g = game();
    let s = schedules(&g, &["org", "kaple"]);
    let refs: Vec<&Schedule> = s.iter().collect();
    insta::assert_snapshot!(organizer_sheets_tex(&g, &refs, None).unwrap());
}

#[test]
fn sheets_are_deterministic() {
    let render = || {
        let g = game();
        let (ann, bob) = (id("ann"), id("bob"));
        let d = destinies(&g, &[&ann, &bob]);
        let s = schedules(&g, &["org", "kaple"]);
        let refs: Vec<&Schedule> = s.iter().collect();
        [
            player_sheet_html(&g, &ann, &d[0], None).unwrap(),
            player_sheets_tex(&g, &[(&ann, &d[0]), (&bob, &d[1])], None).unwrap(),
            organizer_sheet_html(&g, &s[0], None).unwrap(),
            organizer_sheets_tex(&g, &refs, None).unwrap(),
        ]
    };
    let first = render();
    for _ in 0..5 {
        assert_eq!(render(), first);
    }
}
