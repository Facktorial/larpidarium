//! Property tests: `Id` parsing rules and `load` never panicking on arbitrary file contents.

use larp_model::{Checked, Id, IdError, Severity, SourceFile, load};
use proptest::prelude::*;

/// Valid kebab-case ids: a letter, then letters and digits, in hyphen-separated groups.
/// At most 9 + 6 * 9 = 63 characters, inside [`Id::MAX_LEN`].
fn valid_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,8}(-[a-z0-9]{1,8}){0,6}"
}

/// The documented id rules, written independently of `Id::parse`.
fn follows_rules(s: &str) -> bool {
    let b = s.as_bytes();
    !s.is_empty()
        && s.len() <= Id::MAX_LEN
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        && !s.contains("--")
        && !s.ends_with('-')
}

fn insert_at(s: &str, at: usize, what: char) -> String {
    let at = at % (s.len() + 1);
    format!("{}{what}{}", &s[..at], &s[at..])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn id_accepts_and_round_trips_valid_kebab_case(s in valid_id()) {
        let id = Id::parse(&s).unwrap();
        prop_assert_eq!(id.as_str(), s.as_str());
        prop_assert_eq!(id.to_string(), s.clone());
        prop_assert_eq!(Id::try_from(s.clone()).unwrap(), id);
    }

    #[test]
    fn id_rejects_an_uppercase_letter_anywhere(
        s in valid_id(),
        at in any::<usize>(),
        upper in prop::char::range('A', 'Z'),
    ) {
        let bad = insert_at(&s, at, upper);
        prop_assert!(Id::parse(&bad).is_err(), "{:?} was accepted", bad);
    }

    #[test]
    fn id_rejects_a_space_anywhere(s in valid_id(), at in any::<usize>()) {
        let bad = insert_at(&s, at, ' ');
        prop_assert!(Id::parse(&bad).is_err(), "{:?} was accepted", bad);
    }

    #[test]
    fn id_rejects_a_leading_hyphen(s in valid_id()) {
        prop_assert_eq!(Id::parse(&format!("-{s}")), Err(IdError::BadStart));
    }

    #[test]
    fn id_rejects_a_trailing_hyphen(s in valid_id()) {
        let bad = format!("{s}-");
        prop_assume!(bad.len() <= Id::MAX_LEN);
        prop_assert_eq!(Id::parse(&bad), Err(IdError::BadHyphen));
    }

    #[test]
    fn id_rejects_non_ascii_letters(s in valid_id(), at in any::<usize>(),
        c in prop::sample::select(vec!['ř', 'ů', 'č', 'é', 'ß'])) {
        let bad = insert_at(&s, at, c);
        prop_assert!(Id::parse(&bad).is_err(), "{:?} was accepted", bad);
    }

    #[test]
    fn id_parse_accepts_exactly_the_documented_rules(s in "[a-zA-Z0-9 _ř-]{0,70}") {
        let parsed = Id::parse(&s);
        prop_assert_eq!(parsed.is_ok(), follows_rules(&s), "{:?} -> {:?}", s, parsed);
    }
}

// ---------- load ----------

const GAME: &str = "[game]\ntitle = \"Řád\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\"]\n";
const SEGMENTS: &str =
    "[[segment]]\nid = \"one\"\nname = \"Noc 1\"\n\n[[segment]]\nid = \"two\"\nname = \"Noc 2\"\n";
const CYCLES: &str =
    "[[cycle]]\nid = \"c\"\nname = \"Cyklus\"\nhero = \"ann\"\nfrom = \"one\"\nto = \"two\"\n";

fn run(files: &[(&str, &str)]) -> Checked {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    load(&files)
}

/// What `load` promises: a game exactly when there are no errors, and diagnostics whenever
/// there is no game.
fn assert_consistent(checked: &Checked) -> Result<(), TestCaseError> {
    let errors = checked.count(Severity::Error);
    prop_assert_eq!(
        checked.game.is_some(),
        errors == 0,
        "{:?}",
        checked.diagnostics
    );
    if checked.game.is_none() {
        prop_assert!(!checked.diagnostics.is_empty());
        prop_assert!(checked.playthroughs.is_none());
    }
    // Errors come first.
    let first_non_error = checked
        .diagnostics
        .iter()
        .position(|d| d.severity != Severity::Error)
        .unwrap_or(checked.diagnostics.len());
    prop_assert_eq!(first_non_error, errors);
    Ok(())
}

/// Lines that look like game or person TOML, valid and invalid, so the generated files often
/// parse and reach validation.
fn toml_line() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec![
            "[game]",
            "title = \"Řád\"",
            "title = 5",
            "language = \"cs\"",
            "language = \"en\"",
            "language = \"xx\"",
            "steps = []",
            "steps = [\"Ty\", \"Potřeba\", \"Odchod\"]",
            "[person]",
            "name = \"Čeněk Ůžasný\"",
            "name = \"\"",
            "role = \"player\"",
            "role = \"npc\"",
            "role = \"organizer\"",
            "role = \"king\"",
            "color = \"#c0392b\"",
            "color = \"#zz\"",
            "goals = [\"Přežít\"]",
            "secrets = []",
            "dies = 0",
            "dies = 1",
            "dies = 2",
            "dies = 255",
            "dies = \"one\"",
            "dies = \"nowhere\"",
            "enters = \"two\"",
            "enters = \"Bad Id\"",
            "after = \"ann\"",
            "after = \"bob\"",
            "played_by = \"p1\"",
            "outside_cycle = true",
            "[person.sheet]",
            "backstory = \"Řeč\"",
            "[[person.phase]]",
            "step = 0",
            "step = 2",
            "step = 9",
            "segment = \"two\"",
            "segment = \"nope\"",
            "[[person.relation]]",
            "with = \"bob\"",
            "with = \"ann\"",
            "text = \"Bratr\"",
            "[[",
            "]",
            "= \"x\"",
            "\"unterminated",
            "x = 1\nx = 2",
        ])
        .prop_map(str::to_owned),
        "\\PC{0,20}",
    ]
}

fn toml_text() -> impl Strategy<Value = String> {
    prop_oneof![
        // A valid person header followed by more lines, so validation runs on odd lives.
        prop::collection::vec(toml_line(), 0..4).prop_map(|lines| format!(
            "[person]\nname = \"Ann\"\nrole = \"npc\"\n{}",
            lines.join("\n")
        )),
        prop::collection::vec(toml_line(), 0..14).prop_map(|lines| lines.join("\n")),
        any::<String>(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn load_returns_diagnostics_for_arbitrary_game_toml(text in toml_text()) {
        let checked = run(&[("game.toml", &text), ("segments.toml", SEGMENTS)]);
        assert_consistent(&checked)?;
    }

    #[test]
    fn load_returns_diagnostics_for_arbitrary_person_file(
        ann in toml_text(),
    ) {
        let checked = run(&[
            ("game.toml", GAME),
            ("segments.toml", SEGMENTS),
            ("cycles.toml", CYCLES),
            ("players.toml", "[[player]]\nid = \"p1\"\nname = \"Pat\"\n"),
            ("people/ann.toml", &ann),
            ("people/bob.toml", "[person]\nname = \"Bob\"\nrole = \"npc\"\n"),
        ]);
        assert_consistent(&checked)?;
    }

    #[test]
    fn load_reports_an_error_for_a_person_file_that_is_not_a_person(text in any::<String>()) {
        // Without a `[person]` table the file can never be a valid person.
        prop_assume!(!text.contains("person"));
        let checked = run(&[("game.toml", GAME), ("people/ann.toml", &text)]);
        assert_consistent(&checked)?;
        prop_assert!(checked.game.is_none());
        prop_assert!(
            checked.diagnostics.iter().any(|d| d.severity == Severity::Error),
            "{:?}",
            checked.diagnostics
        );
    }
}
