//! Shared text blocks in the graph data: expanded texts, texts as written, `blocks` and progress.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{GraphData, OpenPart, build, progress};
use larp_model::{Game, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n";

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

const BLOCKS: &str = "[[block]]\nid = \"zvesti\"\ntitle = \"Zvěsti z přístavu\"\n\
    text = \"Šeptá se, že [[don]] platí do [[datum]].\"\nshow = [\"ann\", \"don\"]\n\n\
    [[block]]\nid = \"datum\"\ntext = \"13. března\"\n\n\
    [[block]]\nid = \"jen-show\"\ntext = \"Bez odkazů.\"\nshow = [\"ann\"]\n";

/// A game where some texts have links and others don't.
fn linked() -> GraphData {
    let g = game(&[
        ("blocks.toml", BLOCKS),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\n",
        ),
        (
            "people/don.toml",
            "[person]\nname = \"Don Řehoř\"\nrole = \"npc\"\ngoals = [\"Do [[datum]] zmizet.\"]\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"s1\"\nname = \"Ráno\"\ntext = \"Začíná [[datum]].\"\n\n\
             [[segment]]\nid = \"s2\"\nname = \"Večer\"\ntext = \"Bez odkazů.\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Klíč\"\nstarting = true\ntext = \"Patří [[don]].\"\n\n\
             [[item]]\nid = \"mec\"\nname = \"Meč\"\nstarting = true\ntext = \"Obyčejný.\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"zprava\"\nname = \"Zpráva\"\nstarting = true\ntext = \"Od [[don]] a znovu [[don]].\"\n",
        ),
        (
            "actions/s-odkazem.toml",
            "[action]\ntitle = \"S odkazem\"\nowners = [\"ann\"]\nneeds = [\"klic\", \"mec\", \"zprava\"]\n\
             text = \"Pro orgy: [[zvesti]]\"\nbrief = \"Víš: [[datum]].\"\n",
        ),
        (
            "actions/bez-odkazu.toml",
            "[decision]\ntitle = \"Bez odkazu\"\nowners = [\"ann\"]\ntext = \"Prostý text.\"\nbrief = \"Prostý brief.\"\n\n\
             [[decision.option]]\nlabel = \"Ano\"\n",
        ),
        (
            "actions/jen-brief.toml",
            "[action]\ntitle = \"Jen brief\"\nowners = [\"ann\"]\ntext = \"Prostý text.\"\nbrief = \"[[don]]\"\n",
        ),
    ]);
    build(&g, &[], None)
}

fn node<'a>(data: &'a GraphData, id: &str) -> &'a larp_graph::NodeView {
    data.nodes.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn node_texts_are_expanded() {
    let data = linked();
    let n = node(&data, "s-odkazem");
    assert_eq!(
        n.text,
        "Pro orgy: Šeptá se, že Don Řehoř platí do 13. března."
    );
    assert_eq!(n.brief, "Víš: 13. března.");
}

#[test]
fn node_keeps_texts_as_written_when_links_changed_them() {
    let data = linked();
    let n = node(&data, "s-odkazem");
    assert_eq!(n.raw_text.as_deref(), Some("Pro orgy: [[zvesti]]"));
    assert_eq!(n.raw_brief.as_deref(), Some("Víš: [[datum]]."));
}

#[test]
fn node_without_links_has_no_raw_texts() {
    let data = linked();
    let n = node(&data, "bez-odkazu");
    assert_eq!(
        (n.raw_text.as_deref(), n.raw_brief.as_deref()),
        (None, None)
    );
}

#[test]
fn node_keeps_only_the_raw_text_that_differs() {
    let data = linked();
    let n = node(&data, "jen-brief");
    assert_eq!(n.raw_text, None);
    assert_eq!(n.raw_brief.as_deref(), Some("[[don]]"));
    assert_eq!(n.brief, "Don Řehoř");
}

#[test]
fn things_keep_the_raw_text_only_when_links_changed_it() {
    let data = linked();
    let thing = |id: &str| data.things.iter().find(|t| t.id == id).unwrap();
    assert_eq!(thing("klic").text, "Patří Don Řehoř.");
    assert_eq!(thing("klic").raw_text.as_deref(), Some("Patří [[don]]."));
    assert_eq!(thing("zprava").text, "Od Don Řehoř a znovu Don Řehoř.");
    assert_eq!(
        thing("zprava").raw_text.as_deref(),
        Some("Od [[don]] a znovu [[don]].")
    );
    assert_eq!(thing("mec").raw_text, None);
}

#[test]
fn segments_keep_the_raw_text_only_when_links_changed_it() {
    let data = linked();
    let segment = |id: &str| data.segments.iter().find(|s| s.id == id).unwrap();
    assert_eq!(segment("s1").text, "Začíná 13. března.");
    assert_eq!(segment("s1").raw_text.as_deref(), Some("Začíná [[datum]]."));
    assert_eq!(segment("s2").raw_text, None);
}

#[test]
fn blocks_list_every_block_in_file_order() {
    let data = linked();
    let ids: Vec<&str> = data.blocks.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["zvesti", "datum", "jen-show"]);
}

#[test]
fn block_view_has_expanded_and_raw_text_title_and_show() {
    let data = linked();
    let b = &data.blocks[0];
    assert_eq!(b.title, "Zvěsti z přístavu");
    assert_eq!(b.text, "Šeptá se, že Don Řehoř platí do 13. března.");
    assert_eq!(b.raw_text, "Šeptá se, že [[don]] platí do [[datum]].");
    assert_eq!(b.show, ["ann", "don"]);
}

#[test]
fn block_view_without_title_has_an_empty_title() {
    let data = linked();
    assert_eq!(data.blocks[1].title, "");
    assert_eq!(data.blocks[1].raw_text, data.blocks[1].text);
}

#[test]
fn block_used_in_lists_each_place_once() {
    let data = linked();
    let datum = &data.blocks[1];
    // `[[datum]]` appears in an action brief, a goal, a segment and another block.
    assert_eq!(
        datum.used_in,
        [
            "actions/s-odkazem.toml (brief)",
            "people/don.toml (goals / secrets)",
            "segments.toml (text)",
            "blocks.toml (text)",
        ]
    );
}

#[test]
fn block_used_twice_in_one_place_is_listed_once() {
    let g = game(&[
        (
            "blocks.toml",
            "[[block]]\nid = \"datum\"\ntext = \"13. března\"\n",
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\ngoals = [\"[[datum]]\", \"Zase [[datum]] a [[datum]]\"]\n",
        ),
    ]);
    let data = build(&g, &[], None);
    assert_eq!(
        data.blocks[0].used_in,
        ["people/ann.toml (goals / secrets)"]
    );
}

#[test]
fn block_only_shown_has_no_uses() {
    let data = linked();
    assert_eq!(data.blocks[2].used_in, Vec::<String>::new());
    assert_eq!(data.blocks[2].show, ["ann"]);
}

#[test]
fn game_without_blocks_has_empty_blocks() {
    let g = game(&[]);
    assert!(build(&g, &[], None).blocks.is_empty());
}

fn tracked_blocks() -> Game {
    game(&[
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\n",
        ),
        (
            "blocks.toml",
            "[[block]]\nid = \"zvesti\"\ntitle = \"Zvěsti\"\ntext = \"x\"\nshow = [\"ann\"]\nstatus = \"draft\"\nwriter = \"Jana\"\n\n\
             [[block]]\nid = \"datum\"\ntext = \"y\"\nshow = [\"ann\"]\nstatus = \"review\"\n\n\
             [[block]]\nid = \"hotovo\"\ntext = \"z\"\nshow = [\"ann\"]\nstatus = \"done\"\nwriter = \"Jana\"\n\n\
             [[block]]\nid = \"nic\"\ntext = \"w\"\nshow = [\"ann\"]\n",
        ),
    ])
}

#[test]
fn progress_counts_blocks_by_status() {
    let p = progress(&tracked_blocks());
    // Four blocks and Ann (untracked).
    assert_eq!(
        (p.all.draft, p.all.review, p.all.done, p.all.untracked),
        (1, 1, 1, 2)
    );
}

#[test]
fn progress_counts_blocks_per_writer() {
    let p = progress(&tracked_blocks());
    assert_eq!(p.writers.len(), 1);
    let (name, count) = &p.writers[0];
    assert_eq!(name, "Jana");
    assert_eq!((count.draft, count.done, count.total()), (1, 1, 2));
}

#[test]
fn progress_lists_open_blocks_by_title_falling_back_to_the_id() {
    let p = progress(&tracked_blocks());
    assert_eq!(
        p.open,
        [
            OpenPart {
                kind: "block",
                id: "zvesti".into(),
                title: "Zvěsti".into(),
                status: "draft",
                writer: Some("Jana".into()),
            },
            OpenPart {
                kind: "block",
                id: "datum".into(),
                title: "datum".into(),
                status: "review",
                writer: None,
            },
        ]
    );
}

#[test]
fn block_view_carries_status_and_writer() {
    let data = build(&tracked_blocks(), &[], None);
    let b = &data.blocks[0];
    assert_eq!(
        (b.status, b.writer.as_deref()),
        (Some("draft"), Some("Jana"))
    );
    assert_eq!(data.blocks[3].status, None);
}
