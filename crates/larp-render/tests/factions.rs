//! Factions on sheets: the "Tvá frakce" section of player sheets (HTML and LaTeX), blocks shown to
//! a faction, and the label of a faction's organizer sheet.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_model::{Game, Id, SourceFile, load};
use larp_render::{organizer_sheet_html, organizer_sheets_tex};

/// Names and texts need HTML and LaTeX escaping; the sheet section key `history` has no label, so
/// it is printed as written.
const FACTIONS: &str = "[[faction]]\nid = \"montekove\"\nname = \"Montekové & <spol.>\"\ncolor = \"#2c3e50\"\n\
    goals = [\"Udržet přístav <navždy> & 100 %\"]\n\
    secrets = [\"Otec prodal sklad za $5_000 #1\"]\n\n\
    [faction.sheet]\nhistory = \"Řád {starý} drží [[julie]] v šachu.\"\n";

const BLOCKS: &str = "[[block]]\nid = \"heslo\"\ntitle = \"Rodové heslo\"\ntext = \"Čest & krev.\"\nshow = [\"montekove\"]\n";

fn person(name: &str, role: &str, factions: &str) -> String {
    format!("[person]\nname = \"{name}\"\nrole = \"{role}\"\nfactions = [{factions}]\n")
}

/// Romeo, Benvolio (players) and Tybalt Čapek (NPC) are Montekové; Julie is not. Expanded, as the
/// binary passes it to the renderer.
fn game(language: &str) -> Game {
    let files = [
        (
            "game.toml",
            format!("[game]\ntitle = \"Verona\"\nlanguage = \"{language}\"\n"),
        ),
        ("factions.toml", FACTIONS.to_owned()),
        ("blocks.toml", BLOCKS.to_owned()),
        (
            "people/romeo.toml",
            person("Romeo", "player", "\"montekove\""),
        ),
        (
            "people/benvolio.toml",
            person("Benvolio", "player", "\"montekove\""),
        ),
        (
            "people/tybalt.toml",
            person("Tybalt Čapek", "npc", "\"montekove\""),
        ),
        (
            "people/julie.toml",
            person("Julie Kapuletová", "player", ""),
        ),
        (
            "actions/boj.toml",
            "[decision]\ntitle = \"Boj na náměstí\"\nowners = [\"montekove\", \"julie\"]\n\n\
             [[decision.option]]\nlabel = \"Tasit\"\n"
                .to_owned(),
        ),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: t.clone(),
        })
        .collect();
    let checked = load(&files);
    let raw = checked
        .game
        .unwrap_or_else(|| panic!("{:?}", checked.diagnostics));
    larp_model::text::expand(&raw)
}

fn html(language: &str, person: &str) -> String {
    let g = game(language);
    let id = Id::parse(person).unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    larp_render::player_sheet_html(&g, &id, &destiny, None).unwrap()
}

fn tex(language: &str, person: &str) -> String {
    let g = game(language);
    let id = Id::parse(person).unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    larp_render::player_sheets_tex(&g, &[(&id, &destiny)], None).unwrap()
}

/// The part of `out` from `start` up to the next `end` (or the end of `out`).
fn section<'a>(out: &'a str, start: &str, end: &str) -> &'a str {
    let from = out
        .find(start)
        .unwrap_or_else(|| panic!("no `{start}` in {out}"));
    let rest = &out[from..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |i| i + start.len());
    &rest[..to]
}

// ---------- player sheet, HTML ----------

#[test]
fn html_sheet_of_a_member_has_the_faction_heading_with_the_escaped_name() {
    let out = html("cs", "romeo");
    assert!(out.contains("Tvá frakce: "), "{out}");
    assert!(out.contains("Montekové &amp; &lt;spol.&gt;"), "{out}");
    assert!(!out.contains("Montekové & <spol.>"), "{out}");
}

#[test]
fn html_sheet_names_the_other_members_but_not_the_person() {
    let out = html("cs", "romeo");
    let members = section(&out, "Další členové", "</p>");
    assert!(members.contains("Benvolio, Tybalt Čapek"), "{members}");
    assert!(!members.contains("Romeo"), "{members}");
}

#[test]
fn html_sheet_of_another_member_names_romeo() {
    let out = html("cs", "tybalt");
    let members = section(&out, "Další členové", "</p>");
    assert!(members.contains("Benvolio, Romeo"), "{members}");
    assert!(!members.contains("Tybalt"), "{members}");
}

#[test]
fn html_sheet_shows_shared_goals_escaped() {
    let out = html("cs", "romeo");
    assert!(out.contains("<h4>Společné cíle</h4>"), "{out}");
    assert!(
        out.contains("<li>Udržet přístav &lt;navždy&gt; &amp; 100 %</li>"),
        "{out}"
    );
}

#[test]
fn html_sheet_shows_shared_secrets() {
    let out = html("cs", "romeo");
    assert!(out.contains("<h4>Společná tajemství</h4>"), "{out}");
    assert!(
        out.contains("<li>Otec prodal sklad za $5_000 #1</li>"),
        "{out}"
    );
}

#[test]
fn html_sheet_shows_faction_sheet_sections_with_links_expanded() {
    let out = html("cs", "romeo");
    assert!(out.contains("<h4>history</h4>"), "{out}");
    assert!(
        out.contains("Řád {starý} drží Julie Kapuletová v šachu."),
        "{out}"
    );
}

#[test]
fn html_sheet_of_a_non_member_has_no_faction_section() {
    let out = html("cs", "julie");
    assert!(!out.contains("Tvá frakce"), "{out}");
    assert!(!out.contains("Společné cíle"), "{out}");
    assert!(!out.contains("Otec prodal sklad"), "{out}");
}

#[test]
fn html_sheet_uses_english_faction_labels() {
    let out = html("en", "romeo");
    assert!(out.contains("Your faction: "), "{out}");
    assert!(out.contains("Other members"), "{out}");
    assert!(out.contains("Shared goals"), "{out}");
    assert!(out.contains("Shared secrets"), "{out}");
}

#[test]
fn html_sheet_of_the_only_member_has_no_members_line() {
    let g = {
        let files = [
            (
                "game.toml",
                "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n".to_owned(),
            ),
            (
                "factions.toml",
                "[[faction]]\nid = \"cech\"\nname = \"Cech\"\ngoals = [\"Přežít\"]\n".to_owned(),
            ),
            ("people/ann.toml", person("Ann", "player", "\"cech\"")),
        ];
        let files: Vec<SourceFile> = files
            .iter()
            .map(|(p, t)| SourceFile {
                path: (*p).to_owned(),
                text: t.clone(),
            })
            .collect();
        load(&files).game.unwrap()
    };
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    let out = larp_render::player_sheet_html(&g, &id, &destiny, None).unwrap();
    assert!(out.contains("Tvá frakce: "), "{out}");
    assert!(!out.contains("Další členové"), "{out}");
}

#[test]
fn faction_owned_decision_is_in_a_members_html_destiny() {
    let out = html("cs", "benvolio");
    assert!(out.contains("Boj na náměstí"), "{out}");
}

// ---------- player sheet, LaTeX ----------

#[test]
fn tex_sheet_of_a_member_has_the_faction_heading_with_the_escaped_name() {
    let out = tex("cs", "romeo");
    assert!(
        out.contains(r"\subhead{Tvá frakce: Montekové \& <spol.>}"),
        "{out}"
    );
}

#[test]
fn tex_sheet_names_the_other_members_but_not_the_person() {
    let out = tex("cs", "romeo");
    let members = section(&out, "Další členové", r"\par");
    assert!(members.contains("Benvolio, Tybalt Čapek"), "{members}");
    assert!(!members.contains("Romeo"), "{members}");
}

#[test]
fn tex_sheet_shows_shared_goals_escaped() {
    let out = tex("cs", "romeo");
    assert!(out.contains("Společné cíle"), "{out}");
    assert!(
        out.contains(r"\item Udržet přístav <navždy> \& 100 \%"),
        "{out}"
    );
}

#[test]
fn tex_sheet_shows_shared_secrets_escaped() {
    let out = tex("cs", "romeo");
    assert!(out.contains("Společná tajemství"), "{out}");
    assert!(
        out.contains(r"\item Otec prodal sklad za \$5\_000 \#1"),
        "{out}"
    );
}

#[test]
fn tex_sheet_shows_faction_sheet_sections_escaped() {
    let out = tex("cs", "romeo");
    assert!(out.contains(r"{\bfseries history}\par"), "{out}");
    assert!(
        out.contains(r"Řád \{starý\} drží Julie Kapuletová v šachu.\par"),
        "{out}"
    );
}

#[test]
fn tex_sheet_of_a_non_member_has_no_faction_section() {
    let out = tex("cs", "julie");
    assert!(!out.contains("Tvá frakce"), "{out}");
    assert!(!out.contains("Otec prodal sklad"), "{out}");
}

// ---------- blocks shown to a faction ----------

#[test]
fn block_shown_to_a_faction_is_on_every_members_html_sheet() {
    for member in ["romeo", "benvolio", "tybalt"] {
        let out = html("cs", member);
        assert!(out.contains("<h3>Rodové heslo</h3>"), "{member}: {out}");
        assert!(out.contains("Čest &amp; krev."), "{member}: {out}");
    }
}

#[test]
fn block_shown_to_a_faction_is_not_on_a_non_members_sheet() {
    let out = html("cs", "julie");
    assert!(!out.contains("Rodové heslo"), "{out}");
    assert!(!out.contains("Čest"), "{out}");
}

#[test]
fn block_shown_to_a_faction_is_on_a_members_tex_sheet() {
    let out = tex("cs", "romeo");
    assert!(out.contains(r"\subhead{Rodové heslo}"), "{out}");
    assert!(out.contains(r"Čest \& krev."), "{out}");
}

// ---------- organizer sheet of a faction ----------

#[test]
fn organizer_html_sheet_of_a_faction_is_labeled_frakce() {
    let g = game("cs");
    let s = larp_graph::schedule(&g, &Id::parse("montekove").unwrap());
    let out = organizer_sheet_html(&g, &s, None).unwrap();
    assert!(out.contains(r#"<p class="role">frakce</p>"#), "{out}");
    assert!(out.contains("Montekové &amp; &lt;spol.&gt;"), "{out}");
    assert!(out.contains("Boj na náměstí"), "{out}");
}

#[test]
fn organizer_html_sheet_of_a_faction_is_labeled_faction_in_english() {
    let g = game("en");
    let s = larp_graph::schedule(&g, &Id::parse("montekove").unwrap());
    let out = organizer_sheet_html(&g, &s, None).unwrap();
    assert!(out.contains(r#"<p class="role">faction</p>"#), "{out}");
}

#[test]
fn organizer_tex_sheet_of_a_faction_is_labeled_frakce() {
    let g = game("cs");
    let s = larp_graph::schedule(&g, &Id::parse("montekove").unwrap());
    let out = organizer_sheets_tex(&g, &[&s], None).unwrap();
    assert!(out.contains(r"\textcolor{muted}{frakce}"), "{out}");
    assert!(out.contains(r"Montekové \& <spol.>"), "{out}");
}

#[test]
fn organizer_sheet_of_a_person_is_not_labeled_frakce() {
    let g = game("cs");
    let s = larp_graph::schedule(&g, &Id::parse("tybalt").unwrap());
    let out = organizer_sheet_html(&g, &s, None).unwrap();
    assert!(!out.contains(r#"<p class="role">frakce</p>"#), "{out}");
    assert!(out.contains(r#"<p class="role">cizí postava</p>"#), "{out}");
}

// ---------- faction color ----------

/// The colors of the person's factions on the sheet, rendered with a tiny template.
fn faction_colors(color: &str) -> String {
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n".to_owned(),
        ),
        (
            "factions.toml",
            format!("[[faction]]\nid = \"cech\"\nname = \"Cech\"\ncolor = \"{color}\"\n"),
        ),
        ("people/ann.toml", person("Ann", "npc", "\"cech\"")),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: t.clone(),
        })
        .collect();
    let checked = load(&files);
    // A bad color is an error, so only the draft has it; the renderer must still be safe.
    let g = checked.game.or(checked.draft).unwrap();
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    let template = "{% for f in person.factions %}[{{ f.color }}]{% endfor %}";
    larp_render::player_sheet_html(&g, &id, &destiny, Some(template)).unwrap()
}

#[test]
fn faction_color_is_passed_to_the_sheet_when_it_is_hex() {
    assert_eq!(faction_colors("#2c3e50"), "[#2c3e50]");
}

#[test]
fn faction_color_is_empty_on_the_sheet_when_it_is_not_hex() {
    assert_eq!(faction_colors("red; background: url(x)"), "[]");
    assert_eq!(faction_colors("#2c3e5"), "[]");
}
