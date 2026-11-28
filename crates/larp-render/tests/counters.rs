//! The counter line ("Počitadla" / "Counters") on organizer sheets, in HTML and LaTeX, and
//! `tex_escape` of the signs it uses.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::Schedule;
use larp_model::{Game, Id, SourceFile, load};
use larp_render::{organizer_sheet_html, organizer_sheets_tex, tex_escape};
use proptest::prelude::*;

/// Petra runs `bribe` (requires and changes a counter whose name needs escaping) and `verdict`
/// (a decision with one option that requires it); Tom runs `plain`, which has no counters.
fn game(language: &str) -> Game {
    let files = [
        (
            "game.toml",
            format!("[game]\ntitle = \"Opatství\"\nlanguage = \"{language}\"\n"),
        ),
        (
            "counters.toml",
            "[[counter]]\nid = \"suspicion\"\nname = \"Podezření <ř> & 50 %_#\"\n".to_owned(),
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n".to_owned(),
        ),
        (
            "people/tom.toml",
            "[person]\nname = \"Tom\"\nrole = \"npc\"\n".to_owned(),
        ),
        (
            "actions/bribe.toml",
            "[action]\ntitle = \"Úplatek\"\nowners = [\"petra\"]\nchange = { suspicion = -1 }\nrequire = { suspicion = { at_least = 3 } }\n"
                .to_owned(),
        ),
        (
            "actions/verdict.toml",
            "[decision]\ntitle = \"Verdikt\"\nowners = [\"petra\"]\n\n[[decision.option]]\nlabel = \"Obvinit\"\nrequire = { suspicion = { at_most = 1 } }\n"
                .to_owned(),
        ),
        (
            "actions/plain.toml",
            "[action]\ntitle = \"Úklid\"\nowners = [\"tom\"]\n".to_owned(),
        ),
    ];
    let files: Vec<SourceFile> = files
        .into_iter()
        .map(|(p, text)| SourceFile {
            path: p.to_owned(),
            text,
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

fn sheet(g: &Game, owner: &str) -> Schedule {
    larp_graph::schedule(g, &Id::parse(owner).unwrap())
}

fn html(language: &str, owner: &str) -> String {
    let g = game(language);
    organizer_sheet_html(&g, &sheet(&g, owner), None).unwrap()
}

fn tex(language: &str, owner: &str) -> String {
    let g = game(language);
    let s = sheet(&g, owner);
    organizer_sheets_tex(&g, &[&s], None).unwrap()
}

// ---------- HTML ----------

#[test]
fn html_shows_the_counter_line_of_an_action_escaped() {
    let out = html("cs", "petra");
    assert!(
        out.contains(
            "<b>Počitadla:</b> Podezření &lt;ř&gt; &amp; 50 %_# ≥ 3, Podezření &lt;ř&gt; &amp; 50 %_# −1</p>"
        ),
        "{out}"
    );
    assert!(!out.contains("<ř>"), "the counter name is escaped");
}

#[test]
fn html_shows_the_counter_line_of_a_decision_option() {
    let out = html("cs", "petra");
    assert!(
        out.contains("<b>Počitadla:</b> Podezření &lt;ř&gt; &amp; 50 %_# ≤ 1</p>"),
        "{out}"
    );
}

#[test]
fn html_counter_label_is_english_for_an_english_game() {
    let out = html("en", "petra");
    assert!(out.contains("<b>Counters:</b> Podezření"), "{out}");
    assert!(!out.contains("Počitadla"));
}

#[test]
fn html_has_no_counter_line_without_counters() {
    let out = html("cs", "tom");
    assert!(out.contains("Úklid"), "{out}");
    assert!(!out.contains("Počitadla"), "{out}");
}

// ---------- LaTeX ----------

#[test]
fn tex_shows_the_counter_line_of_an_action_escaped() {
    let out = tex("cs", "petra");
    let name = r"Podezření <ř> \& 50 \%\_\#";
    let line = format!(
        r"\flowline{{Počitadla}}{{{name} \ensuremath{{\geq}} 3, {name} \ensuremath{{-}}1}}"
    );
    assert!(out.contains(&line), "missing {line} in {out}");
}

#[test]
fn tex_shows_the_counter_line_of_a_decision_option() {
    let out = tex("cs", "petra");
    let line = r"\flowline{Počitadla}{Podezření <ř> \& 50 \%\_\# \ensuremath{\leq} 1}";
    assert!(out.contains(line), "missing {line} in {out}");
}

#[test]
fn tex_counter_line_never_has_raw_signs() {
    let out = tex("cs", "petra");
    assert!(!out.contains(['≥', '≤', '−']), "{out}");
    assert!(!out.contains("50 %"), "{out}");
}

#[test]
fn tex_counter_label_is_english_for_an_english_game() {
    let out = tex("en", "petra");
    assert!(out.contains(r"\flowline{Counters}{Podezření"), "{out}");
}

#[test]
fn tex_has_no_counter_line_without_counters() {
    let out = tex("cs", "tom");
    assert!(out.contains("Úklid"), "{out}");
    assert!(!out.contains("Počitadla"), "{out}");
}

// ---------- tex_escape of ≥ ≤ − ----------

#[test]
fn tex_escape_typesets_greater_or_equal_as_math() {
    assert_eq!(tex_escape("≥"), r"\ensuremath{\geq}");
}

#[test]
fn tex_escape_typesets_less_or_equal_as_math() {
    assert_eq!(tex_escape("≤"), r"\ensuremath{\leq}");
}

#[test]
fn tex_escape_typesets_the_minus_sign_as_math() {
    assert_eq!(tex_escape("−"), r"\ensuremath{-}");
}

#[test]
fn tex_escape_keeps_hyphen_and_en_dash() {
    assert_eq!(tex_escape("Peníze 1–4, -2"), "Peníze 1–4, -2");
}

#[test]
fn tex_escape_handles_signs_next_to_czech_text() {
    assert_eq!(
        tex_escape("Podezření≥3 Řád≤1 Číslo−2"),
        r"Podezření\ensuremath{\geq}3 Řád\ensuremath{\leq}1 Číslo\ensuremath{-}2"
    );
}

fn signs_and_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            3 => prop::sample::select(vec!['≥', '≤', '−', '-', '–', '{', '}', '\\', ' ']),
            2 => prop::sample::select(vec!['ř', 'ů', 'č', 'Ř']),
            2 => prop::char::range('0', '9'),
            1 => any::<char>(),
        ],
        0..40,
    )
    .prop_map(|v| v.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn tex_escape_never_leaves_a_raw_comparison_or_minus_sign(input in signs_and_text()) {
        let out = tex_escape(&input);
        prop_assert!(!out.contains(['≥', '≤', '−']), "{:?} -> {:?}", input, out);
    }

    #[test]
    fn tex_escape_writes_one_math_sign_per_input_sign(input in signs_and_text()) {
        let out = tex_escape(&input);
        let count = |c: char| input.chars().filter(|&x| x == c).count();
        prop_assert_eq!(out.matches(r"\ensuremath{\geq}").count(), count('≥'));
        prop_assert_eq!(out.matches(r"\ensuremath{\leq}").count(), count('≤'));
        prop_assert_eq!(out.matches(r"\ensuremath{-}").count(), count('−'));
    }
}
