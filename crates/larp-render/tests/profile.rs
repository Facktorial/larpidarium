//! The character profile page: escaping, labels in both languages, the spoiler notice, empty
//! sections, a custom template and a snapshot.
//!
//! Review snapshot changes with `cargo insta review`; accept them only when the new output is
//! intended.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_graph::{Moment, Profile};
use larp_model::{Game, Id, SourceFile, load};
use larp_render::{RenderError, profile_html};

fn load_game(files: &[(&str, &str)]) -> Game {
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

/// A game with nothing in it but its title and language, for hand-built profiles.
fn bare_game(language: &str) -> Game {
    let game = format!("[game]\ntitle = \"Řád & <Kříž>\"\nlanguage = \"{language}\"\n");
    load_game(&[("game.toml", &game)])
}

/// Text that HTML must escape, with Czech letters, tagged with `tag` so each field is findable.
fn raw(tag: &str) -> String {
    format!("{tag} & <ř> \"ů\" č")
}

/// How [`raw`] looks once escaped.
fn escaped(tag: &str) -> String {
    format!("{tag} &amp; &lt;ř&gt; &quot;ů&quot; č")
}

fn empty_moment(kind: &'static str) -> Moment {
    Moment {
        when: String::new(),
        kind,
        node: None,
        title: String::new(),
        brief: String::new(),
        text: String::new(),
        with: Vec::new(),
        options: Vec::new(),
        finds: Vec::new(),
        told: Vec::new(),
        gets: Vec::new(),
        uses: Vec::new(),
        at: None,
        end: None,
    }
}

fn empty_profile() -> Profile {
    Profile {
        id: "ann".to_owned(),
        name: "Anna".to_owned(),
        role: "player",
        color: String::new(),
        played_by: String::new(),
        factions: Vec::new(),
        goals: Vec::new(),
        secrets: Vec::new(),
        sheet: Vec::new(),
        relations: Vec::new(),
        seen_by: Vec::new(),
        starts_with: Vec::new(),
        timeline: Vec::new(),
        status: "",
        writer: String::new(),
    }
}

/// Every text field filled with [`raw`], each with its own tag, and one moment of every kind.
fn full_profile() -> Profile {
    let node = Moment {
        when: raw("WHEN"),
        node: Some("volba".to_owned()),
        title: raw("TITLE"),
        brief: raw("BRIEF"),
        text: raw("NOTES"),
        with: vec![raw("WITH")],
        options: vec![raw("OPTION")],
        finds: vec![raw("FINDS")],
        told: vec![raw("TOLD")],
        gets: vec![raw("GETS")],
        uses: vec![raw("USES")],
        at: Some(0),
        ..empty_moment("decision")
    };
    let phase = Moment {
        when: raw("PHASEWHEN"),
        brief: raw("PHASEGOALS"),
        text: raw("PHASESECRETS"),
        ..empty_moment("phase")
    };
    Profile {
        name: raw("NAME"),
        played_by: raw("PLAYEDBY"),
        factions: vec![raw("FACTION")],
        goals: vec![raw("GOAL")],
        secrets: vec![raw("SECRET")],
        sheet: vec![
            ("appearance".to_owned(), raw("SHEETTEXT")),
            (raw("SHEETKEY"), raw("OTHER")),
        ],
        relations: vec![(raw("RELWHO"), raw("RELTEXT"))],
        seen_by: vec![(raw("SEENWHO"), raw("SEENTEXT"))],
        starts_with: vec![raw("START")],
        timeline: vec![empty_moment("enters"), phase, node, empty_moment("dies")],
        status: "draft",
        writer: raw("WRITER"),
        ..empty_profile()
    }
}

const TAGS: [&str; 26] = [
    "NAME",
    "PLAYEDBY",
    "FACTION",
    "GOAL",
    "SECRET",
    "SHEETTEXT",
    "SHEETKEY",
    "OTHER",
    "RELWHO",
    "RELTEXT",
    "SEENWHO",
    "SEENTEXT",
    "START",
    "WRITER",
    "WHEN",
    "TITLE",
    "BRIEF",
    "NOTES",
    "WITH",
    "OPTION",
    "FINDS",
    "TOLD",
    "GETS",
    "USES",
    "PHASEGOALS",
    "PHASESECRETS",
];

fn render(language: &str, p: &Profile) -> String {
    profile_html(&bare_game(language), p, None).unwrap()
}

// ---------- escaping ----------

#[test]
fn escapes_every_text_field_and_keeps_czech_letters() {
    let html = render("cs", &full_profile());
    for tag in TAGS {
        assert!(html.contains(&escaped(tag)), "{tag} not escaped:\n{html}");
    }
    assert!(html.contains(&escaped("PHASEWHEN")), "{html}");
}

#[test]
fn leaves_no_raw_user_markup_in_the_page() {
    let html = render("cs", &full_profile());
    assert!(!html.contains("<ř>"), "{html}");
    assert!(!html.contains("\"ů\""), "{html}");
    assert!(!html.contains("& <"), "{html}");
}

#[test]
fn escapes_the_game_title() {
    let html = render("cs", &empty_profile());
    assert!(html.contains("Řád &amp; &lt;Kříž&gt;"), "{html}");
    assert!(!html.contains("<Kříž>"), "{html}");
}

// ---------- labels ----------

#[test]
fn uses_czech_labels_in_a_czech_game() {
    let html = render("cs", &full_profile());
    assert!(html.contains("<html lang=\"cs\">"), "{html}");
    for label in [
        "Profil postavy",
        "hráčská postava",
        "hraje",
        "Tvá frakce",
        "rozepsané",
        "Na začátku hry",
        "Cíle",
        "Tajemství",
        "Má u sebe a ví",
        "Časová osa",
        "Vstupuje do hry",
        "Nová fáze listu",
        "rozhodnutí",
        "S kým",
        "Volby",
        "Zjistí",
        "Dozví se",
        "Získá",
        "Spotřebuje",
        "Umírá",
        "Vztahy",
        "Co si postava myslí o ostatních",
        "Co si o postavě myslí ostatní",
        "Části listu",
        "Vzhled",
    ] {
        assert!(html.contains(label), "missing {label:?}:\n{html}");
    }
    assert!(!html.contains("Character profile"), "{html}");
}

#[test]
fn uses_english_labels_in_an_english_game() {
    let html = render("en", &full_profile());
    assert!(html.contains("<html lang=\"en\">"), "{html}");
    for label in [
        "Character profile",
        "player character",
        "played by",
        "Your faction",
        "draft",
        "When the game starts",
        "Goals",
        "Secrets",
        "Has and knows",
        "Timeline",
        "Enters the game",
        "New sheet phase",
        "decision",
        "With",
        "Options",
        "Finds out",
        "Is told",
        "Gets",
        "Uses up",
        "Dies",
        "Relationships",
        "What they think of others",
        "What others think of them",
        "Sheet sections",
        "Appearance",
    ] {
        assert!(html.contains(label), "missing {label:?}:\n{html}");
    }
    assert!(!html.contains("Profil postavy"), "{html}");
}

#[test]
fn names_organizer_and_npc_roles() {
    let org = Profile {
        role: "organizer",
        ..empty_profile()
    };
    assert!(render("cs", &org).contains("organizátor"));
    let npc = Profile {
        role: "npc",
        ..empty_profile()
    };
    assert!(render("en", &npc).contains("non-player character"));
}

#[test]
fn names_each_writing_status() {
    for (status, cs, en) in [
        ("draft", "rozepsané", "draft"),
        ("review", "ke kontrole", "in review"),
        ("done", "hotovo", "done"),
    ] {
        let p = Profile {
            status,
            ..empty_profile()
        };
        assert!(render("cs", &p).contains(cs), "{status}");
        assert!(render("en", &p).contains(en), "{status}");
    }
}

#[test]
fn prints_an_unlabelled_sheet_key_as_written() {
    let p = Profile {
        sheet: vec![("vůně".to_owned(), "Česnek".to_owned())],
        ..empty_profile()
    };
    let html = render("cs", &p);
    assert!(html.contains("<h3>vůně</h3>"), "{html}");
}

#[test]
fn shows_no_time_for_a_moment_without_when() {
    let p = Profile {
        timeline: vec![Moment {
            title: "Volno".to_owned(),
            ..empty_moment("action")
        }],
        ..empty_profile()
    };
    assert!(render("cs", &p).contains("bez času"));
    assert!(render("en", &p).contains("no time"));
}

// ---------- spoiler notice ----------

#[test]
fn always_shows_the_spoiler_notice() {
    let cs = render("cs", &empty_profile());
    assert!(
        cs.contains("Pro organizátory: obsahuje celý příběh postavy včetně tajemství a konců."),
        "{cs}"
    );
    let en = render("en", &empty_profile());
    // The apostrophe is escaped too.
    assert!(
        en.contains(
            "For organizers: contains the character&#x27;s whole story, secrets and endings included."
        ),
        "{en}"
    );
}

// ---------- empty sections ----------

#[test]
fn renders_no_section_headings_for_an_empty_profile() {
    let html = render("en", &empty_profile());
    assert!(!html.contains("<h2>"), "{html}");
    assert!(!html.contains("<h3>"), "{html}");
    assert!(!html.contains("<ol class=\"timeline\">"), "{html}");
    for label in ["When the game starts", "Relationships", "Sheet sections"] {
        assert!(!html.contains(label), "{label}:\n{html}");
    }
    // No played-by, faction, status or writer facts either.
    assert!(!html.contains("played by"), "{html}");
    assert!(!html.contains("Your faction"), "{html}");
    assert!(!html.contains('✎'), "{html}");
}

#[test]
fn renders_only_the_filled_parts_of_a_section() {
    let p = Profile {
        goals: vec!["Přežít".to_owned()],
        seen_by: vec![("Bob".to_owned(), "Bojí se jí".to_owned())],
        ..empty_profile()
    };
    let html = render("en", &p);
    assert!(html.contains("<h2>When the game starts</h2>"), "{html}");
    assert!(html.contains("<h3>Goals</h3>"), "{html}");
    assert!(!html.contains("Secrets"), "{html}");
    assert!(!html.contains("Has and knows"), "{html}");
    assert!(html.contains("What others think of them"), "{html}");
    assert!(!html.contains("What they think of others"), "{html}");
}

#[test]
fn renders_only_the_filled_lines_of_a_node_moment() {
    let p = Profile {
        timeline: vec![Moment {
            title: "Hledání".to_owned(),
            when: "Noc".to_owned(),
            ..empty_moment("action")
        }],
        ..empty_profile()
    };
    let html = render("en", &p);
    assert!(html.contains("Hledání"), "{html}");
    for label in [
        "With:",
        "Options:",
        "Finds out:",
        "Is told:",
        "Gets:",
        "Uses up:",
        "class=\"brief\"",
        "class=\"notes\"",
        "decision",
    ] {
        assert!(!html.contains(label), "{label}:\n{html}");
    }
}

// ---------- custom template ----------

#[test]
fn uses_a_custom_template_instead_of_the_built_in_one() {
    let p = full_profile();
    let html = profile_html(
        &bare_game("en"),
        &p,
        Some("CUSTOM {{ t.profile_title }}: {{ p.name }} ({{ p.timeline | length }})"),
    )
    .unwrap();
    assert_eq!(
        html,
        format!("CUSTOM Character profile: {} (4)", escaped("NAME"))
    );
}

#[test]
fn fails_with_a_template_error_on_a_broken_custom_template() {
    let err = profile_html(&bare_game("cs"), &empty_profile(), Some("{% if %}")).unwrap_err();
    assert!(matches!(err, RenderError::Template(_)), "{err:?}");
}

// ---------- snapshot ----------

/// A small game where Ann has every part of a profile: a player, a faction, a cycle with a phase
/// and a death, a room, items and info. Texts are Czech and contain characters HTML must escape.
fn snapshot_game() -> Game {
    load_game(&[
        (
            "game.toml",
            "[game]\ntitle = \"Řád & Kříž\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\"]\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"noc\"\nname = \"Noc <1>\"\n\n[[segment]]\nid = \"rano\"\nname = \"Ráno\"\n",
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
            "factions.toml",
            "[[faction]]\nid = \"rad\"\nname = \"Řád & spol.\"\n",
        ),
        ("rooms.toml", "[[room]]\nid = \"kaple\"\nname = \"Kaple\"\n"),
        (
            "people/ann.toml",
            "[person]\nname = \"Anna <Nováková>\"\nrole = \"player\"\ncolor = \"#c0392b\"\nplayed_by = \"p1\"\n\
             factions = [\"rad\"]\ndies = 2\nstatus = \"review\"\nwriter = \"Petra\"\n\
             goals = [\"Přežít & utéct\"]\nsecrets = [\"Ukradla \\\"klíč\\\"\"]\n\n\
             [person.sheet]\nwhereabouts = \"V kapli\"\nbackstory = \"Kuchařka.\\n\\nDruhý odstavec.\"\n\n\
             [[person.relation]]\nwith = \"bob\"\ntext = \"Bratr <starší>\"\n\n\
             [[person.phase]]\nstep = 2\ngoals = [\"Utéct\"]\nsecrets = [\"Zradila\"]\n",
        ),
        (
            "people/bob.toml",
            "[person]\nname = \"Bob Čech\"\nrole = \"player\"\n\n[[person.relation]]\nwith = \"ann\"\ntext = \"Bojí se jí\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Klíč\"\nowners = [\"ann\"]\nstarting = true\n\n\
             [[item]]\nid = \"svice\"\nname = \"Svíce\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"heslo\"\nname = \"Heslo\"\nowners = [\"rad\"]\nstarting = true\n\n\
             [[info]]\nid = \"konec\"\nname = \"Konec\"\nending = true\n",
        ),
        (
            "actions/a-hledani.toml",
            "[action]\ntitle = \"Hledání v kapli\"\ncycle = \"anin\"\nstep = 1\nowners = [\"ann\", \"bob\", \"kaple\"]\n\
             needs = [\"klic\", \"heslo\"]\ngives = [\"svice\"]\nbrief = \"Hledáš světlo.\"\ntext = \"Org: schovej svíci & čekej.\"\n",
        ),
        (
            "actions/b-volba.toml",
            "[decision]\ntitle = \"Volba\"\ncycle = \"anin\"\nstep = 2\nowners = [\"rad\"]\nbrief = \"Co uděláš?\"\n\n\
             [[decision.option]]\nlabel = \"Zůstat\"\nuses = [\"svice\"]\ngives = [\"konec\"]\n\n\
             [[decision.option]]\nlabel = \"Odejít\"\ngives = [\"konec\"]\n",
        ),
    ])
}

#[test]
fn profile_html_matches_snapshot() {
    let g = snapshot_game();
    let p = larp_graph::profile(&g, &Id::parse("ann").unwrap()).unwrap();
    insta::assert_snapshot!(profile_html(&g, &p, None).unwrap());
}
