//! Blocks with `show` as sections on player sheets, in HTML and LaTeX.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_model::{Game, Id, SourceFile, load};

const BLOCKS: &str = "[[block]]\nid = \"zvesti\"\ntitle = \"Zvěsti <z> přístavu & 50 %\"\n\
    text = \"Šeptá se, že [[bob]] platí <stráž> & 100 % \\\\ #1.\"\nshow = [\"ann\"]\n\n\
    [[block]]\nid = \"fakt\"\ntext = \"Řeka teče na sever_.\"\nshow = [\"ann\"]\n";

const ANN: &str = "[person]\nname = \"Ann\"\nrole = \"player\"\ngoals = [\"x\"]\n\n\
    [person.sheet]\nwhereabouts = \"x\"\nbackstory = \"x\"\n";
const BOB: &str = "[person]\nname = \"Bob Čáp\"\nrole = \"player\"\ngoals = [\"x\"]\n\n\
    [person.sheet]\nwhereabouts = \"x\"\nbackstory = \"x\"\n";

/// The game with links expanded, as the binary passes it to the renderer.
fn game(language: &str) -> Game {
    let files = [
        (
            "game.toml",
            format!("[game]\ntitle = \"Test\"\nlanguage = \"{language}\"\n"),
        ),
        ("people/ann.toml", ANN.to_owned()),
        ("people/bob.toml", BOB.to_owned()),
        ("blocks.toml", BLOCKS.to_owned()),
        // Ann carries two things, so a block section nested in the `carries` list would repeat.
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Klíč\"\nowners = [\"ann\"]\nstarting = true\nending = true\n\n\
             [[item]]\nid = \"mec\"\nname = \"Meč\"\nowners = [\"ann\"]\nstarting = true\nending = true\n"
                .to_owned(),
        ),
        (
            "actions/talk.toml",
            "[decision]\ntitle = \"Talk\"\nowners = [\"ann\", \"bob\"]\n\n[[decision.option]]\nlabel = \"Yes\"\n"
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

#[test]
fn html_sheet_shows_a_block_under_its_escaped_title() {
    let out = html("cs", "ann");
    assert!(
        out.contains("<h3>Zvěsti &lt;z&gt; přístavu &amp; 50 %</h3>"),
        "{out}"
    );
}

#[test]
fn html_sheet_shows_the_expanded_escaped_block_text() {
    let out = html("cs", "ann");
    assert!(
        out.contains("Šeptá se, že Bob Čáp platí &lt;stráž&gt; &amp; 100 % \\ #1."),
        "{out}"
    );
    assert!(!out.contains("[[bob]]"), "{out}");
}

#[test]
fn html_sheet_titles_an_untitled_block_with_the_shared_label() {
    let cs = html("cs", "ann");
    assert!(cs.contains("<h3>Co víš s ostatními</h3>"), "{cs}");
    assert!(cs.contains("Řeka teče na sever_."), "{cs}");
    let en = html("en", "ann");
    assert!(en.contains("<h3>What you share with others</h3>"), "{en}");
}

#[test]
fn html_sheet_of_a_player_not_in_show_has_no_block() {
    let out = html("cs", "bob");
    assert!(!out.contains("Zvěsti"), "{out}");
    assert!(!out.contains("Co víš s ostatními"), "{out}");
    assert!(!out.contains("Řeka teče"), "{out}");
}

#[test]
fn html_sheet_shows_blocks_in_file_order() {
    let out = html("cs", "ann");
    let first = out.find("Zvěsti").unwrap();
    let second = out.find("Co víš s ostatními").unwrap();
    assert!(first < second);
}

#[test]
fn tex_sheet_shows_a_block_under_its_escaped_title() {
    let out = tex("cs", "ann");
    assert!(
        out.contains(r"\subhead{Zvěsti <z> přístavu \& 50 \%}"),
        "{out}"
    );
}

#[test]
fn tex_sheet_shows_the_expanded_escaped_block_text() {
    let out = tex("cs", "ann");
    assert!(
        out.contains(r"Šeptá se, že Bob Čáp platí <stráž> \& 100 \% \textbackslash{} \#1."),
        "{out}"
    );
    assert!(out.contains(r"Řeka teče na sever\_."), "{out}");
}

#[test]
fn tex_sheet_titles_an_untitled_block_with_the_shared_label() {
    let out = tex("cs", "ann");
    assert!(out.contains(r"\subhead{Co víš s ostatními}"), "{out}");
}

#[test]
fn tex_sheet_shows_each_block_once() {
    let out = tex("cs", "ann");
    assert!(
        out.contains("Meč"),
        "Ann must carry things for this test: {out}"
    );
    assert_eq!(out.matches("Řeka teče").count(), 1, "{out}");
}

#[test]
fn tex_sheet_of_a_player_not_in_show_has_no_block() {
    let out = tex("cs", "bob");
    assert!(!out.contains("Zvěsti"), "{out}");
    assert!(!out.contains("Co víš s ostatními"), "{out}");
}
