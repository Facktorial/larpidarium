//! Rendering player sheets.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_model::{Game, Id, SourceFile, load};

fn game(person: &str) -> Game {
    let files = [
        ("game.toml", "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n"),
        ("people/ann.toml", person),
        (
            "actions/talk.toml",
            "[decision]\ntitle = \"Talk\"\nowners = [\"ann\"]\nbrief = \"Speak up.\"\n\n[[decision.option]]\nlabel = \"Yes\"\n",
        ),
    ];
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: (*t).to_owned(),
        })
        .collect();
    load(&files).game.unwrap()
}

fn render(person: &str) -> String {
    let g = game(person);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    larp_render::player_sheet_html(&g, &id, &destiny, None).unwrap()
}

const ANN: &str = "[person]\nname = \"Anna Nováková\"\nrole = \"player\"\ngoals = [\"Přežít\"]\n\n[person.sheet]\nwhereabouts = \"V kuchyni.\"\nbackstory = \"Kuchařka.\"\nsmell = \"Česnek\"\n";

#[test]
fn sheet_has_both_parts_in_czech() {
    let html = render(ANN);
    assert!(html.contains("Anna Nováková"));
    assert!(html.contains("Kdo jsi"));
    assert!(html.contains("Tvůj osud"));
    assert!(html.contains("V kuchyni."));
    assert!(html.contains("Speak up."));
    assert!(html.contains("Tvé rozhodnutí"));
}

#[test]
fn unknown_sheet_keys_become_extra_sections() {
    let html = render(ANN);
    assert!(html.contains(">smell</h3>"));
    assert!(html.contains("Česnek"));
}

#[test]
fn user_text_is_html_escaped() {
    let evil = ANN.replace("Kuchařka.", "<script>alert(1)</script>");
    let html = render(&evil);
    assert!(!html.contains("<script>alert(1)"));
    assert!(html.contains("&lt;script&gt;"));
}

#[test]
fn custom_template_is_used() {
    let g = game(ANN);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    let html =
        larp_render::player_sheet_html(&g, &id, &destiny, Some("{{ person.name }}|{{ t.part2 }}"))
            .unwrap();
    assert_eq!(html, "Anna Nováková|Tvůj osud");
}

#[test]
fn broken_template_is_a_clear_error() {
    let g = game(ANN);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    let err = larp_render::player_sheet_html(&g, &id, &destiny, Some("{% if %}")).unwrap_err();
    assert!(err.to_string().starts_with("template error"));
}
