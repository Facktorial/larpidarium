//! Factions in the graph data: destinies, organizer schedules, owners, progress; and relations in
//! the viewer's owner list.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_graph::{OwnerView, Schedule, build, destiny, progress, schedule, schedule_owners};
use larp_model::{Game, Id, SourceFile, load};

const GAME: &str = "[game]\ntitle = \"Verona\"\nlanguage = \"cs\"\n";
const FACTIONS: &str = "[[faction]]\nid = \"montekove\"\nname = \"Montekové\"\ncolor = \"#2c3e50\"\n\
    status = \"draft\"\nwriter = \"Petra\"\n";

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = std::iter::once(("game.toml", GAME))
        .chain(files.iter().copied())
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        })
        .collect();
    let checked = load(&files);
    let diagnostics = format!("{:?}", checked.diagnostics);
    checked
        .game
        .unwrap_or_else(|| panic!("game did not load: {diagnostics}"))
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

/// Romeo (player) and Tybalt (NPC) are Montekové, Julie (player) is not; Petra organizes.
///
/// The faction owns `boj` (a decision) and holds the starting item `mec`; Romeo owns `rande` with
/// Julie; Tybalt owns `strazit` alone; Petra owns `uvod`.
fn verona() -> Game {
    game(&[
        ("factions.toml", FACTIONS),
        (
            "people/romeo.toml",
            "[person]\nname = \"Romeo\"\nrole = \"player\"\nfactions = [\"montekove\"]\n\n\
             [[person.relation]]\nwith = \"julie\"\ntext = \"Miluješ ji.\"\n\n\
             [[person.relation]]\nwith = \"tybalt\"\ntext = \"Bratranec, kterého se bojíš.\"\n",
        ),
        (
            "people/julie.toml",
            "[person]\nname = \"Julie Kapuletová\"\nrole = \"player\"\n\n\
             [[person.relation]]\nwith = \"romeo\"\ntext = \"Je to Montek, ale…\"\n",
        ),
        (
            "people/tybalt.toml",
            "[person]\nname = \"Tybalt\"\nrole = \"npc\"\nfactions = [\"montekove\"]\n",
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"pristav\"\nname = \"Přístav\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"mec\"\nname = \"Rodový meč\"\nowners = [\"montekove\"]\nstarting = true\nending = true\n",
        ),
        (
            "actions/boj.toml",
            "[decision]\ntitle = \"Boj na náměstí\"\nowners = [\"montekove\"]\n\n[[decision.option]]\nlabel = \"Tasit\"\n",
        ),
        (
            "actions/rande.toml",
            "[decision]\ntitle = \"Rande\"\nowners = [\"romeo\", \"julie\"]\n\n[[decision.option]]\nlabel = \"Ano\"\n",
        ),
        (
            "actions/strazit.toml",
            "[action]\ntitle = \"Hlídka\"\nowners = [\"tybalt\"]\n",
        ),
        (
            "actions/uvod.toml",
            "[action]\ntitle = \"Úvod\"\nowners = [\"petra\"]\n",
        ),
    ])
}

fn step_ids(g: &Game, person: &str) -> Vec<String> {
    let graph = build(g, &[], None);
    let mut ids: Vec<String> = destiny(g, &graph, &id(person))
        .steps
        .into_iter()
        .map(|s| s.id)
        .collect();
    ids.sort();
    ids
}

fn schedule_ids(s: &Schedule) -> Vec<&str> {
    let mut ids: Vec<&str> = s
        .blocks
        .iter()
        .flat_map(|b| &b.steps)
        .map(|st| st.id.as_str())
        .collect();
    ids.sort_unstable();
    ids
}

fn owner<'a>(owners: &'a [OwnerView], who: &str) -> &'a OwnerView {
    owners
        .iter()
        .find(|o| o.id == who)
        .unwrap_or_else(|| panic!("no owner `{who}`"))
}

// ---------- destiny ----------

#[test]
fn destiny_of_a_member_includes_faction_owned_actions() {
    assert_eq!(step_ids(&verona(), "romeo"), ["boj", "rande"]);
}

#[test]
fn destiny_of_every_member_includes_faction_owned_actions() {
    assert_eq!(step_ids(&verona(), "tybalt"), ["boj", "strazit"]);
}

#[test]
fn destiny_of_a_non_member_excludes_faction_owned_actions() {
    assert_eq!(step_ids(&verona(), "julie"), ["rande"]);
}

#[test]
fn faction_owned_node_is_own_in_a_members_mini_graph() {
    let g = verona();
    let graph = build(&g, &[], None);
    let d = destiny(&g, &graph, &id("romeo"));
    let boj = d.graph.nodes.iter().find(|n| n.id == "boj").unwrap();
    assert!(boj.own);
    assert_eq!(boj.label, "Boj na náměstí");
}

#[test]
fn member_carries_a_starting_item_owned_by_the_faction() {
    let g = verona();
    let graph = build(&g, &[], None);
    let carries: Vec<String> = destiny(&g, &graph, &id("romeo"))
        .carries
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(carries, ["Rodový meč"]);
    let julie = destiny(&g, &graph, &id("julie"));
    assert!(julie.carries.is_empty(), "{:?}", julie.carries);
}

// ---------- schedules ----------

#[test]
fn factions_get_an_organizer_schedule() {
    let g = verona();
    let ids: Vec<&str> = schedule_owners(&g).into_iter().map(Id::as_str).collect();
    assert_eq!(ids, ["montekove", "petra", "pristav", "tybalt"]);
}

#[test]
fn faction_schedule_has_kind_faction_name_and_color() {
    let s = schedule(&verona(), &id("montekove"));
    assert_eq!(s.id, "montekove");
    assert_eq!(s.name, "Montekové");
    assert_eq!(s.kind, "faction");
    assert_eq!(s.role, None);
    assert_eq!(s.color, "#2c3e50");
}

#[test]
fn faction_schedule_lists_only_the_nodes_the_faction_owns() {
    let s = schedule(&verona(), &id("montekove"));
    assert_eq!(schedule_ids(&s), ["boj"]);
    assert_eq!(s.load.total, 1);
    assert_eq!(s.load.decisions, 1);
}

#[test]
fn faction_schedule_prepares_items_the_faction_starts_with() {
    let s = schedule(&verona(), &id("montekove"));
    let names: Vec<&str> = s.prepare.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Rodový meč"]);
}

#[test]
fn member_schedule_includes_faction_owned_nodes() {
    let s = schedule(&verona(), &id("tybalt"));
    assert_eq!(s.kind, "person");
    assert_eq!(s.role, Some("npc"));
    assert_eq!(schedule_ids(&s), ["boj", "strazit"]);
}

#[test]
fn non_member_schedule_excludes_faction_owned_nodes() {
    let s = schedule(&verona(), &id("petra"));
    assert_eq!(schedule_ids(&s), ["uvod"]);
}

#[test]
fn faction_without_a_color_has_an_empty_schedule_color() {
    let g = game(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\n",
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\nfactions = [\"cech\"]\n",
        ),
    ]);
    let s = schedule(&g, &id("cech"));
    assert_eq!(s.kind, "faction");
    assert_eq!(s.color, "");
    assert!(s.blocks.is_empty(), "{:?}", s.blocks);
}

// ---------- owners in the viewer ----------

#[test]
fn faction_owner_view_has_kind_faction_and_its_members() {
    let owners = build(&verona(), &[], None).owners;
    let f = owner(&owners, "montekove");
    assert_eq!(f.kind, "faction");
    assert_eq!(f.name, "Montekové");
    assert_eq!(f.role, None);
    assert_eq!(f.color, "#2c3e50");
    assert_eq!(f.members, ["romeo", "tybalt"]);
    assert!(f.factions.is_empty(), "{:?}", f.factions);
    assert_eq!(f.status, Some("draft"));
    assert_eq!(f.writer.as_deref(), Some("Petra"));
}

#[test]
fn people_owner_views_list_their_factions() {
    let owners = build(&verona(), &[], None).owners;
    assert_eq!(owner(&owners, "romeo").factions, ["montekove"]);
    assert_eq!(owner(&owners, "tybalt").factions, ["montekove"]);
    assert!(
        owner(&owners, "julie").factions.is_empty(),
        "{:?}",
        owner(&owners, "julie").factions
    );
    assert!(
        owner(&owners, "romeo").members.is_empty(),
        "{:?}",
        owner(&owners, "romeo").members
    );
}

#[test]
fn rooms_have_no_factions_or_members() {
    let owners = build(&verona(), &[], None).owners;
    let room = owner(&owners, "pristav");
    assert_eq!(room.kind, "room");
    assert!(room.factions.is_empty(), "{:?}", room.factions);
    assert!(room.members.is_empty(), "{:?}", room.members);
}

#[test]
fn faction_owner_view_comes_after_people_and_rooms() {
    let owners = build(&verona(), &[], None).owners;
    let kinds: Vec<&str> = owners.iter().map(|o| o.kind).collect();
    assert_eq!(
        kinds,
        ["person", "person", "person", "person", "room", "faction"]
    );
}

// ---------- relations in the viewer ----------

#[test]
fn person_owner_view_lists_relations_as_with_and_text_pairs() {
    let owners = build(&verona(), &[], None).owners;
    assert_eq!(
        owner(&owners, "romeo").relations,
        [
            ("julie".to_owned(), "Miluješ ji.".to_owned()),
            (
                "tybalt".to_owned(),
                "Bratranec, kterého se bojíš.".to_owned()
            ),
        ]
    );
}

#[test]
fn person_without_relations_has_none() {
    let owners = build(&verona(), &[], None).owners;
    assert!(
        owner(&owners, "tybalt").relations.is_empty(),
        "{:?}",
        owner(&owners, "tybalt").relations
    );
}

#[test]
fn rooms_and_factions_have_no_relations() {
    let owners = build(&verona(), &[], None).owners;
    assert!(
        owner(&owners, "pristav").relations.is_empty(),
        "{:?}",
        owner(&owners, "pristav").relations
    );
    assert!(
        owner(&owners, "montekove").relations.is_empty(),
        "{:?}",
        owner(&owners, "montekove").relations
    );
}

#[test]
fn relation_texts_in_the_viewer_have_links_expanded() {
    let g = game(&[
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\n\n\
             [[person.relation]]\nwith = \"bob\"\ntext = \"Dluží [[bob]] peníze.\"\n",
        ),
        (
            "people/bob.toml",
            "[person]\nname = \"Bořek Čáp\"\nrole = \"npc\"\n",
        ),
    ]);
    let owners = build(&g, &[], None).owners;
    assert_eq!(
        owner(&owners, "ann").relations,
        [("bob".to_owned(), "Dluží Bořek Čáp peníze.".to_owned())]
    );
}

// ---------- progress ----------

#[test]
fn progress_counts_factions() {
    let p = progress(&verona());
    // 4 nodes + 4 people + 1 faction; only the faction has a status.
    assert_eq!(p.all.total(), 9);
    assert_eq!(p.all.draft, 1);
    assert_eq!(p.all.untracked, 8);
}

#[test]
fn progress_lists_an_open_faction_with_kind_faction() {
    let p = progress(&verona());
    assert_eq!(p.open.len(), 1, "{:?}", p.open);
    let open = &p.open[0];
    assert_eq!(open.kind, "faction");
    assert_eq!(open.id, "montekove");
    assert_eq!(open.title, "Montekové");
    assert_eq!(open.status, "draft");
    assert_eq!(open.writer.as_deref(), Some("Petra"));
}

#[test]
fn progress_counts_factions_per_writer() {
    let p = progress(&verona());
    assert_eq!(p.writers.len(), 1);
    assert_eq!(p.writers[0].0, "Petra");
    assert_eq!(p.writers[0].1.draft, 1);
}

#[test]
fn done_faction_is_counted_but_not_open() {
    let g = game(&[
        (
            "factions.toml",
            "[[faction]]\nid = \"cech\"\nname = \"Cech\"\nstatus = \"done\"\n",
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Ann\"\nrole = \"npc\"\nfactions = [\"cech\"]\n",
        ),
    ]);
    let p = progress(&g);
    assert_eq!(p.all.done, 1);
    assert!(p.open.is_empty(), "{:?}", p.open);
    assert!(!p.is_untracked());
}
