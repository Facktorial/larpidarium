//! LaTeX player sheets and escaping.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_model::{Game, Id, SourceFile, load};
use larp_render::{player_sheets_tex, tex_escape};

const SPECIAL: [char; 10] = ['\\', '{', '}', '$', '&', '#', '%', '_', '^', '~'];

fn game(title: &str, ann: &str) -> Game {
    let files = [
        ("game.toml", format!("[game]\ntitle = \"{title}\"\nlanguage = \"cs\"\n")),
        ("people/ann.toml", ann.to_owned()),
        (
            "people/bob.toml",
            "[person]\nname = \"Bob\"\nrole = \"player\"\ngoals = [\"x\"]\n\n[person.sheet]\nwhereabouts = \"x\"\nbackstory = \"x\"\n".to_owned(),
        ),
        (
            "actions/talk.toml",
            "[decision]\ntitle = \"Mluv_50 %\"\nowners = [\"ann\", \"bob\"]\nbrief = \"Řekni {to}.\"\n\n[[decision.option]]\nlabel = \"Ano & ne\"\n".to_owned(),
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
    checked.game.unwrap()
}

const ANN: &str = "[person]\nname = \"Anna \\\\Nováková_#1\"\nrole = \"player\"\ncolor = \"#c0392b\"\ngoals = [\"[Přežít] ~ vždy ^\"]\n\n[person.sheet]\nwhereabouts = \"V kuchyni.\"\nbackstory = \"Kuchařka.\\n\\nDruhý odstavec.\"\n";

fn render(title: &str, ids: &[&str], template: Option<&str>) -> String {
    let g = game(title, ANN);
    let graph = larp_graph::build(&g, &[], None);
    let ids: Vec<Id> = ids.iter().map(|i| Id::parse(i).unwrap()).collect();
    let destinies: Vec<_> = ids
        .iter()
        .map(|id| larp_graph::destiny(&g, &graph, id))
        .collect();
    let people: Vec<_> = ids.iter().zip(&destinies).collect();
    player_sheets_tex(&g, &people, template).unwrap()
}

/// Every special character in `out` is part of an escape sequence produced by `tex_escape`.
fn assert_escaped(input: &str) {
    let out = tex_escape(input);
    let cleaned = out
        .replace(r"\textbackslash{}", "")
        .replace(r"\textasciicircum{}", "")
        .replace(r"\textasciitilde{}", "")
        .replace(r"\textquotedbl{}", "")
        .replace("{[}", "")
        .replace("{]}", "");
    let mut chars = cleaned.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let next = chars.next();
            assert!(
                next.is_some_and(|n| "{}$&#%_".contains(n)),
                "stray backslash in {out:?} (from {input:?})"
            );
        } else {
            assert!(
                !SPECIAL.contains(&c),
                "unescaped {c:?} in {out:?} (from {input:?})"
            );
            assert!(
                c != '[' && c != ']' && c != '"',
                "unescaped {c:?} in {out:?}"
            );
        }
    }
}

#[test]
fn escape_handles_every_special_character() {
    for input in [
        r"\ { } $ & # % _ ^ ~",
        r#"„Žluťoučký kůň“ "quoted" [1] ]["#,
        r"\\{{}}\textbf{x}",
        "a\n\nb",
        "",
        "%%%___###",
        "~^~^\\",
    ] {
        assert_escaped(input);
    }
}

#[test]
fn escape_drops_control_characters_but_keeps_lines_and_tabs() {
    assert_eq!(tex_escape("a\u{7f}b\u{0c}c\td\ne\r"), "abc\td\ne");
}

#[test]
fn escape_keeps_czech_and_paragraphs() {
    assert_eq!(tex_escape("Říjen, „ahoj“\n\nDál"), "Říjen, „ahoj“\n\nDál");
}

#[test]
fn game_text_is_escaped_in_the_document() {
    let tex = render("Hra & spol. 100 %", &["ann"], None);
    assert!(tex.contains(r"Hra \& spol. 100 \%"), "{tex}");
    assert!(tex.contains(r"Anna \textbackslash{}Nováková\_\#1"), "{tex}");
    assert!(
        tex.contains(r"{[}Přežít{]} \textasciitilde{} vždy \textasciicircum{}"),
        "{tex}"
    );
    assert!(tex.contains(r"Mluv\_50 \%"), "{tex}");
    assert!(tex.contains(r"Řekni \{to\}."), "{tex}");
    assert!(tex.contains(r"Ano \& ne"), "{tex}");
    assert!(tex.contains("Kuchařka.\n\nDruhý odstavec."), "{tex}");
}

#[test]
fn document_has_preamble_parts_and_accent() {
    let tex = render("Test", &["ann"], None);
    assert!(tex.starts_with(r"\documentclass"), "{tex}");
    assert!(tex.contains(r"\usepackage[czech]{babel}"));
    assert!(tex.contains(r"\definecolor{accent}{HTML}{C0392B}"));
    assert!(tex.contains(r"\sheetpart{I}{Kdo jsi}"));
    assert!(tex.contains(r"\sheetpart{II}{Tvůj osud}"));
    assert!(tex.contains(r"\begin{tikzpicture}"));
    assert!(tex.trim_end().ends_with(r"\end{document}"));
    assert!(!tex.contains(r"\clearpage"));
}

#[test]
fn several_sheets_share_one_document() {
    let tex = render("Test", &["ann", "bob"], None);
    assert_eq!(tex.matches(r"\documentclass").count(), 1);
    assert_eq!(tex.matches(r"\setcounter{page}{1}").count(), 1);
    assert_eq!(tex.matches(r"\sheetpart{I}").count(), 2);
    // Bob has no color: the default accent.
    assert!(tex.contains(r"\definecolor{accent}{HTML}{7A5C3E}"));
}

#[test]
fn custom_template_is_used_and_escaped() {
    let tpl = "{% for s in sheets %}[{{ s.person.name }}]{% endfor %}";
    let tex = render("Test", &["ann"], Some(tpl));
    assert_eq!(tex, r"[Anna \textbackslash{}Nováková\_\#1]");
}

#[test]
fn unknown_person_is_an_error() {
    let g = game("Test", ANN);
    let graph = larp_graph::build(&g, &[], None);
    let ann = Id::parse("ann").unwrap();
    let destiny = larp_graph::destiny(&g, &graph, &ann);
    let ghost = Id::parse("ghost").unwrap();
    assert!(player_sheets_tex(&g, &[(&ghost, &destiny)], None).is_err());
}

#[test]
fn none_and_undefined_values_print_nothing() {
    // Ann's phases have no `goals` (none); `missing` is undefined.
    let tpl = "[{{ none }}|{{ missing }}|{{ sheets[0].person.played_by }}]";
    assert_eq!(render("Test", &["ann"], Some(tpl)), "[||]");
}

/// No one-argument text macro holds a blank line: LaTeX stops with "Paragraph ended before
/// \text@command was complete".
fn assert_no_paragraph_in_macro(tex: &str) {
    for mac in [r"\textit{", r"\textbf{", r"\textcolor{"] {
        for (at, _) in tex.match_indices(mac) {
            let rest = &tex[at + mac.len()..];
            let end = rest.find('}').unwrap_or(rest.len());
            assert!(
                !rest[..end].contains("\n\n"),
                "blank line inside {mac}…}}: {}",
                &rest[..end]
            );
        }
    }
}

#[test]
fn multi_paragraph_texts_are_never_inside_a_one_argument_macro() {
    let files = [
        ("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n"),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"player\"\n",
        ),
        ("rooms.toml", "[[room]]\nid = \"hall\"\nname = \"Hala\"\n"),
        (
            "actions/talk.toml",
            "[decision]\ntitle = \"Mluv\"\nowners = [\"ann\", \"hall\"]\ntext = \"Org.\\n\\nDruhý.\"\nbrief = \"První.\\n\\nDruhý odstavec.\"\n\n[[decision.option]]\nlabel = \"Ano\"\n",
        ),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: (*t).to_owned(),
        })
        .collect();
    let g = load(&files).game.unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let ann = Id::parse("ann").unwrap();
    let destiny = larp_graph::destiny(&g, &graph, &ann);
    let player = player_sheets_tex(&g, &[(&ann, &destiny)], None).unwrap();
    assert!(player.contains("Druhý odstavec"));
    assert_no_paragraph_in_macro(&player);
    let hall = Id::parse("hall").unwrap();
    let schedule = larp_graph::schedule(&g, &hall);
    let org = larp_render::organizer_sheets_tex(&g, &[&schedule], None).unwrap();
    assert!(org.contains("Druhý odstavec"));
    assert_no_paragraph_in_macro(&org);
}
