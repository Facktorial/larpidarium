//! `blocks.toml` and the checks of `[[id]]` links and `show` lists.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use std::fmt::Write as _;

use larp_model::text::MAX_DEPTH;
use larp_model::{Checked, Diagnostic, Severity, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";
/// An NPC, so that player checks (goals, sheet, decisions) stay quiet.
const ANN: &str = "[person]\nname = \"Anna Nováková\"\nrole = \"npc\"\n";
const ITEMS: &str =
    "[[item]]\nid = \"rusty-key\"\nname = \"Rezavý klíč\"\nstarting = true\nending = true\n";

fn run(files: &[(&str, &str)]) -> Checked {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    load(&files)
}

fn with(checked: &Checked, severity: Severity) -> Vec<&Diagnostic> {
    checked
        .diagnostics
        .iter()
        .filter(|d| d.severity == severity)
        .collect()
}

fn messages(checked: &Checked, severity: Severity) -> Vec<&str> {
    with(checked, severity)
        .into_iter()
        .map(|d| d.message.as_str())
        .collect()
}

/// An action owned by Ann whose `brief` is `brief`.
fn action(brief: &str) -> String {
    format!("[action]\ntitle = \"Akce\"\nowners = [\"ann\"]\nbrief = \"{brief}\"\n")
}

#[test]
fn loads_blocks_with_every_key() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntitle = \"Zvěsti z přístavu\"\ntext = \"Šeptá se.\"\n\
        show = [\"ann\"]\nstatus = \"review\"\nwriter = \"Petra\"\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    let game = checked
        .game
        .unwrap_or_else(|| panic!("{:?}", checked.diagnostics));
    let b = &game.blocks[0];
    assert_eq!(b.id.get_ref().as_str(), "zvesti");
    assert_eq!(b.title.as_deref(), Some("Zvěsti z přístavu"));
    assert_eq!(b.text, "Šeptá se.");
    assert_eq!(b.show[0].get_ref().as_str(), "ann");
    assert_eq!(
        b.status.map(larp_model::model::Status::as_str),
        Some("review")
    );
    assert_eq!(b.writer.as_deref(), Some("Petra"));
}

#[test]
fn blocks_toml_is_not_an_unknown_file() {
    let checked = run(&[("blocks.toml", "")]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn unknown_key_in_a_block_is_an_error() {
    let checked = run(&[(
        "blocks.toml",
        "[[block]]\nid = \"a\"\ntext = \"x\"\nshown = [\"ann\"]\n",
    )]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "blocks.toml");
    assert!(errors[0].message.contains("shown"), "{errors:?}");
}

#[test]
fn used_block_gives_no_diagnostic() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("actions/akce.toml", &action("Víš, že [[datum]].")),
        (
            "blocks.toml",
            "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n",
        ),
    ]);
    let blocky: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.file == "blocks.toml" || d.message.contains("[["))
        .collect();
    assert!(blocky.is_empty(), "{blocky:?}");
    assert!(checked.game.is_some());
}

#[test]
fn block_used_only_inside_another_block_gives_no_warning() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"Do [[datum]].\"\nshow = [\"ann\"]\n\n\
        [[block]]\nid = \"datum\"\ntext = \"13. března\"\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    let in_blocks: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.file == "blocks.toml")
        .collect();
    assert!(in_blocks.is_empty(), "{in_blocks:?}");
}

#[test]
fn shown_block_gives_no_warning() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        (
            "blocks.toml",
            "[[block]]\nid = \"zvesti\"\ntext = \"Šeptá se.\"\nshow = [\"ann\"]\n",
        ),
    ]);
    let in_blocks: Vec<_> = checked
        .diagnostics
        .iter()
        .filter(|d| d.file == "blocks.toml")
        .collect();
    assert!(in_blocks.is_empty(), "{in_blocks:?}");
}

#[test]
fn unused_block_without_show_is_a_warning() {
    let blocks = "[[block]]\nid = \"zapomenuty\"\ntext = \"Nikde.\"\n";
    let checked = run(&[("blocks.toml", blocks)]);
    let warnings = with(&checked, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    let w = warnings[0];
    assert_eq!(w.file, "blocks.toml");
    assert_eq!(w.message, "block `zapomenuty` is not used");
    assert_eq!(&blocks[w.span.clone().unwrap()], "\"zapomenuty\"");
    assert!(checked.game.is_some(), "a warning must not stop the game");
}

#[test]
fn unknown_link_is_an_error_with_a_suggestion() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("actions/akce.toml", &action("Víš, že [[datun]].")),
        (
            "blocks.toml",
            "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n",
        ),
    ]);
    assert!(checked.game.is_none());
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "actions/akce.toml");
    assert_eq!(errors[0].message, "unknown link [[datun]] in brief");
    assert_eq!(errors[0].help.as_deref(), Some("did you mean [[datum]]?"));
}

#[test]
fn unknown_link_suggests_people_rooms_items_and_info_too() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("items.toml", ITEMS),
        ("actions/akce.toml", &action("Máš [[rusty-kez]].")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].help.as_deref(),
        Some("did you mean [[rusty-key]]?")
    );
}

#[test]
fn unknown_link_far_from_any_id_points_to_blocks_toml() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("actions/akce.toml", &action("[[uplne-jine-slovo]]")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].help.as_deref(),
        Some("blocks are defined in blocks.toml")
    );
}

#[test]
fn unknown_link_does_not_suggest_an_action() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("actions/akce.toml", &action("[[akcr]]")),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_ne!(errors[0].help.as_deref(), Some("did you mean [[akce]]?"));
}

#[test]
fn unknown_link_is_found_in_a_person_sheet() {
    let ann = format!("{ANN}\n[person.sheet]\nbackstory = \"Přišla [[nikdy]].\"\n");
    let checked = run(&[("people/ann.toml", &ann)]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "people/ann.toml");
    assert!(errors[0].message.contains("[[nikdy]]"), "{errors:?}");
}

#[test]
fn brackets_around_text_that_is_not_an_id_are_not_checked() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("actions/akce.toml", &action("[[ poznámka ]] [[Velké]]")),
    ]);
    assert!(
        with(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn link_to_an_action_is_an_error() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("actions/akce.toml", &action("Viz [[jina]].")),
        ("actions/jina.toml", &action("x")),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["[[jina]] in brief links to an action"]
    );
    let e = with(&checked, Severity::Error)[0];
    assert_eq!(e.file, "actions/akce.toml");
    assert!(e.help.is_some());
}

#[test]
fn link_to_a_segment_is_an_error() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        (
            "segments.toml",
            "[[segment]]\nid = \"noc\"\nname = \"Noc\"\n",
        ),
        ("actions/akce.toml", &action("V [[noc]].")),
    ]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["[[noc]] in brief links to a segment"]
    );
}

#[test]
fn links_to_people_rooms_items_and_info_are_fine() {
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("items.toml", ITEMS),
        (
            "info.toml",
            "[[info]]\nid = \"zprava\"\nname = \"Zpráva\"\nstarting = true\nending = true\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"krypta\"\nname = \"Krypta\"\n",
        ),
        (
            "actions/akce.toml",
            &action("[[ann]] v [[krypta]] s [[rusty-key]] a [[zprava]]"),
        ),
    ]);
    assert!(
        with(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn block_that_includes_itself_is_an_error() {
    let blocks = "[[block]]\nid = \"sam\"\ntext = \"Já a [[sam]].\"\nshow = [\"ann\"]\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "blocks.toml");
    assert_eq!(errors[0].message, "block `sam` includes itself: sam → sam");
    assert_eq!(&blocks[errors[0].span.clone().unwrap()], "\"sam\"");
}

#[test]
fn blocks_that_include_each_other_are_an_error() {
    let blocks = "[[block]]\nid = \"vejce\"\ntext = \"[[slepice]]\"\nshow = [\"ann\"]\n\n\
        [[block]]\nid = \"slepice\"\ntext = \"[[kohout]]\"\n\n\
        [[block]]\nid = \"kohout\"\ntext = \"[[vejce]]\"\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    let errors = messages(&checked, Severity::Error);
    assert!(
        errors.contains(&"block `vejce` includes itself: vejce → slepice → kohout → vejce"),
        "{errors:?}"
    );
    assert!(checked.game.is_none());
}

#[test]
fn block_included_twice_without_a_loop_is_fine() {
    let blocks = "[[block]]\nid = \"a\"\ntext = \"[[b]] [[c]]\"\nshow = [\"ann\"]\n\n\
        [[block]]\nid = \"b\"\ntext = \"[[c]]\"\n\n\
        [[block]]\nid = \"c\"\ntext = \"list\"\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

/// `blocks.toml` with a chain of `n` blocks, the first one shown to Ann.
fn chain(n: usize) -> String {
    let mut out = String::new();
    for i in 0..n {
        let text = if i + 1 == n {
            "konec".to_owned()
        } else {
            format!("[[b{}]]", i + 1)
        };
        let show = if i == 0 { "show = [\"ann\"]\n" } else { "" };
        writeln!(out, "[[block]]\nid = \"b{i}\"\ntext = \"{text}\"\n{show}").unwrap();
    }
    out
}

#[test]
fn chain_of_max_depth_blocks_is_fine() {
    // A link in a node text is expanded at depth 1, so MAX_DEPTH - 1 blocks still resolve fully.
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("blocks.toml", &chain(MAX_DEPTH - 1)),
    ]);
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
}

#[test]
fn chain_deeper_than_expansion_can_follow_is_an_error() {
    // `expand_text` leaves `[[b16]]` as written in a chain of MAX_DEPTH + 1 blocks, so `check`
    // must not let it reach a sheet silently.
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", &chain(MAX_DEPTH))]);
    assert!(
        !with(&checked, Severity::Error).is_empty(),
        "{:?}",
        checked.diagnostics
    );
}

#[test]
fn unknown_show_entry_is_an_error_with_a_suggestion() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"x\"\nshow = [\"anm\"]\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "blocks.toml");
    assert!(
        errors[0]
            .message
            .starts_with("unknown `show` entry `anm`, expected a person"),
        "{errors:?}"
    );
    assert_eq!(errors[0].help.as_deref(), Some("did you mean `ann`?"));
    assert_eq!(&blocks[errors[0].span.clone().unwrap()], "\"anm\"");
}

#[test]
fn show_entry_that_is_not_a_person_is_an_error() {
    let blocks = "[[block]]\nid = \"zvesti\"\ntext = \"x\"\nshow = [\"rusty-key\"]\n";
    let checked = run(&[("items.toml", ITEMS), ("blocks.toml", blocks)]);
    let errors = messages(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("`rusty-key` is an item, but a `show` entry must be a person"),
        "{errors:?}"
    );
}

#[test]
fn block_with_the_id_of_an_item_is_an_error() {
    let blocks = "[[block]]\nid = \"rusty-key\"\ntext = \"x\"\nshow = [\"ann\"]\n";
    let checked = run(&[
        ("people/ann.toml", ANN),
        ("items.toml", ITEMS),
        ("blocks.toml", blocks),
    ]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "blocks.toml");
    assert_eq!(
        errors[0].message,
        "id `rusty-key` is already used by an item in items.toml"
    );
    assert_eq!(&blocks[errors[0].span.clone().unwrap()], "\"rusty-key\"");
}

#[test]
fn two_blocks_with_one_id_are_an_error() {
    let blocks = "[[block]]\nid = \"a\"\ntext = \"x\"\nshow = [\"ann\"]\n\n\
        [[block]]\nid = \"a\"\ntext = \"y\"\nshow = [\"ann\"]\n";
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", blocks)]);
    assert_eq!(
        messages(&checked, Severity::Error),
        ["id `a` is already used by a block in blocks.toml"]
    );
}

#[test]
fn too_deep_chain_is_found_even_when_a_shortcut_reaches_it_first() {
    // b0 includes b1 (the long way) and b5 (a shortcut into the same chain); the longest chain
    // is still MAX_DEPTH blocks, one too many, and the error names it.
    let blocks = chain(MAX_DEPTH).replacen("text = \"[[b1]]\"", "text = \"[[b1]] [[b5]]\"", 1);
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", &blocks)]);
    let errors = with(&checked, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0]
            .message
            .starts_with("blocks include each other too deep from `b0`"),
        "{errors:?}"
    );
    assert!(!errors[0].message.contains("includes itself"), "{errors:?}");
}

#[test]
fn a_block_that_grows_too_big_when_expanded_is_an_error() {
    // Each level includes the next one ten times: 10^6 copies of a short text.
    let mut blocks = String::new();
    for i in 0..6 {
        let text = format!("[[g{}]] ", i + 1).repeat(10);
        let show = if i == 0 { "show = [\"ann\"]\n" } else { "" };
        writeln!(
            blocks,
            "[[block]]\nid = \"g{i}\"\ntext = \"{text}\"\n{show}"
        )
        .unwrap();
    }
    blocks.push_str("[[block]]\nid = \"g6\"\ntext = \"dlouhý text\"\n");
    let checked = run(&[("people/ann.toml", ANN), ("blocks.toml", &blocks)]);
    let errors = with(&checked, Severity::Error);
    assert!(
        errors
            .iter()
            .any(|e| e.message.starts_with("block `g0` grows to")),
        "{errors:?}"
    );
    assert!(checked.game.is_none());
}
