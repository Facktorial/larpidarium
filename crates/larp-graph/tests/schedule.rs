//! Organizer sheets (`schedule`, `schedule_owners`, `loads`) from small in-memory games.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{ItemRef, Schedule, ScheduleStep, loads, schedule, schedule_owners};
use larp_model::{Game, Id, SourceFile, load};

fn game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

fn id(s: &str) -> Id {
    Id::parse(s).unwrap()
}

/// Three segments; Petra (organizer), Řehoř (NPC), Ann (player), two rooms.
/// File names are chosen so alphabetical order differs from time order.
///
/// Petra: `z-arrive` (Příjezd), `y-ghost` (Noc, with Řehoř and the crypt), `w-verdict`
/// (decision, Ráno, with Ann), `loose` (no segment).
/// Crypt: `x-hide` and `y-ghost`, both in Noc.
fn abbey() -> Game {
    game(&[
        (
            "game.toml",
            "[game]\ntitle = \"Opatství\"\nlanguage = \"cs\"\n",
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"Příjezd\"\n\n[[segment]]\nid = \"two\"\nname = \"Noc\"\n\n[[segment]]\nid = \"three\"\nname = \"Ráno\"\n",
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\ncolor = \"#c0392b\"\n",
        ),
        (
            "people/mnich.toml",
            "[person]\nname = \"Bratr Řehoř\"\nrole = \"npc\"\n",
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Anna\"\nrole = \"player\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"crypt\"\nname = \"Krypta\"\ncolor = \"#2c3e50\"\n\n[[room]]\nid = \"gate\"\nname = \"Brána\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"key\"\nname = \"Klíč\"\nowners = [\"petra\"]\nstarting = true\ntext = \"Starý železný klíč.\"\n\n\
             [[item]]\nid = \"lamp\"\nname = \"Lampa\"\n\n\
             [[item]]\nid = \"candle\"\nname = \"Svíčka\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"clue\"\nname = \"Stopa\"\nowners = [\"mnich\"]\nstarting = true\n\n\
             [[info]]\nid = \"rumor\"\nname = \"Fáma\"\n\n\
             [[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n",
        ),
        (
            "actions/z-arrive.toml",
            "[action]\ntitle = \"Příjezd hráčů\"\nsegment = \"one\"\nowners = [\"petra\"]\ntext = \"Přivítej hráče.\"\ngives = [\"lamp\", \"candle\", \"rumor\"]\n",
        ),
        (
            "actions/y-ghost.toml",
            "[action]\ntitle = \"Duch v kryptě\"\nsegment = \"two\"\nowners = [\"petra\", \"mnich\", \"crypt\"]\nneeds = [\"lamp\", \"clue\", \"rumor\"]\ngives = [\"candle\"]\n",
        ),
        (
            "actions/x-hide.toml",
            "[action]\ntitle = \"Úkryt\"\nsegment = \"two\"\nowners = [\"crypt\", \"ann\"]\nneeds = [\"key\"]\n",
        ),
        (
            "actions/w-verdict.toml",
            "[decision]\ntitle = \"Verdikt\"\nsegment = \"three\"\nowners = [\"petra\", \"ann\"]\n\n\
             [[decision.option]]\nlabel = \"Obvinit\"\nneeds = [\"candle\"]\ngives = [\"end\"]\n\n\
             [[decision.option]]\nlabel = \"Odejít\"\n",
        ),
        (
            "actions/loose.toml",
            "[action]\ntitle = \"Úklid\"\nowners = [\"petra\"]\n",
        ),
    ])
}

/// Three world segments, three steps (3 ticks per segment), Ann's cycle over "two" and "three"
/// (2 ticks per step: step 1 at tick 3, step 2 at 5, both in "two"; step 3 at 7, in "three").
/// Petra runs a world action in "one" and one action per cycle step.
fn cycled() -> Game {
    let action = |title: &str, place: &str| {
        format!("[action]\ntitle = \"{title}\"\nowners = [\"petra\"]\n{place}\n")
    };
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"Koloběh\"\nlanguage = \"cs\"\nsteps = [\"Ty\", \"Potřeba\", \"Odchod\"]\n"
                .to_owned(),
        ),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"Noc 1\"\n\n[[segment]]\nid = \"two\"\nname = \"Noc 2\"\n\n[[segment]]\nid = \"three\"\nname = \"Noc 3\"\n"
                .to_owned(),
        ),
        (
            "players.toml",
            "[[player]]\nid = \"p1\"\nname = \"Pat\"\n".to_owned(),
        ),
        (
            "cycles.toml",
            "[[cycle]]\nid = \"c1\"\nname = \"Annin cyklus\"\nhero = \"ann\"\nfrom = \"two\"\nto = \"three\"\n"
                .to_owned(),
        ),
        (
            "people/ann.toml",
            "[person]\nname = \"Anna Řeháková\"\nrole = \"player\"\nplayed_by = \"p1\"\ndies = 3\n"
                .to_owned(),
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n".to_owned(),
        ),
        ("actions/world.toml", action("Svět", "segment = \"one\"")),
        ("actions/a-step3.toml", action("Krok 3", "cycle = \"c1\"\nstep = 3")),
        ("actions/b-step2.toml", action("Krok 2", "cycle = \"c1\"\nstep = 2")),
        ("actions/c-step1.toml", action("Krok 1", "cycle = \"c1\"\nstep = 1")),
    ];
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    game(&files)
}

fn sheet(g: &Game, owner: &str) -> Schedule {
    schedule(g, &id(owner))
}

fn step<'a>(s: &'a Schedule, node: &str) -> &'a ScheduleStep {
    s.blocks
        .iter()
        .flat_map(|b| &b.steps)
        .find(|st| st.id == node)
        .unwrap()
}

fn find<'a>(list: &'a [ItemRef], thing: &str) -> &'a ItemRef {
    list.iter().find(|x| x.id == thing).unwrap()
}

fn seg(name: &str, n: usize) -> (String, usize) {
    (name.to_owned(), n)
}

// ---------- owners ----------

#[test]
fn schedule_owners_lists_organizers_npcs_and_rooms_by_id_but_not_players() {
    let g = abbey();
    let ids: Vec<String> = schedule_owners(&g)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(ids, ["crypt", "gate", "mnich", "petra"]);
}

#[test]
fn loads_gives_every_owner_name_and_load_in_id_order() {
    let g = abbey();
    let all = loads(&g);
    let names: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Krypta", "Brána", "Bratr Řehoř", "Petra"]);
    assert_eq!(all[3].1, sheet(&g, "petra").load);
    assert_eq!(all[0].1, sheet(&g, "crypt").load);
}

// ---------- load ----------

#[test]
fn load_counts_total_and_decisions() {
    let g = abbey();
    let l = sheet(&g, "petra").load;
    assert_eq!((l.total, l.decisions), (4, 1));
    let crypt = sheet(&g, "crypt").load;
    assert_eq!((crypt.total, crypt.decisions), (2, 0));
}

#[test]
fn load_per_segment_includes_zero_counts_in_segment_order() {
    let g = abbey();
    let l = sheet(&g, "crypt").load;
    assert_eq!(
        l.per_segment,
        [seg("Příjezd", 0), seg("Noc", 2), seg("Ráno", 0)]
    );
}

#[test]
fn load_per_segment_leaves_out_nodes_without_a_segment() {
    let g = abbey();
    let l = sheet(&g, "petra").load;
    assert_eq!(
        l.per_segment,
        [seg("Příjezd", 1), seg("Noc", 1), seg("Ráno", 1)],
        "`loose` counts in total but in no segment"
    );
}

#[test]
fn load_peak_is_the_busiest_segment() {
    let g = abbey();
    assert_eq!(sheet(&g, "crypt").load.peak, Some(seg("Noc", 2)));
}

#[test]
fn load_peak_is_the_first_of_equally_busy_segments() {
    let g = abbey();
    assert_eq!(sheet(&g, "petra").load.peak, Some(seg("Příjezd", 1)));
}

#[test]
fn load_peak_is_none_when_the_owner_has_no_nodes() {
    let g = abbey();
    let l = sheet(&g, "gate").load;
    assert_eq!(l.total, 0);
    assert_eq!(l.peak, None);
    assert_eq!(
        l.per_segment,
        [seg("Příjezd", 0), seg("Noc", 0), seg("Ráno", 0)]
    );
}

#[test]
fn load_peak_is_none_when_the_game_has_no_segments() {
    let g = game(&[
        ("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"en\"\n"),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"end\"\nname = \"End\"\nending = true\n",
        ),
        (
            "actions/go.toml",
            "[action]\ntitle = \"Go\"\nowners = [\"petra\"]\ngives = [\"end\"]\n",
        ),
    ]);
    let l = sheet(&g, "petra").load;
    assert_eq!(l.total, 1);
    assert_eq!(l.per_segment, []);
    assert_eq!(l.peak, None);
}

#[test]
fn load_counts_cycle_nodes_in_the_world_segment_of_their_step() {
    let g = cycled();
    let l = sheet(&g, "petra").load;
    assert_eq!(
        l.per_segment,
        [seg("Noc 1", 1), seg("Noc 2", 2), seg("Noc 3", 1)]
    );
    assert_eq!(l.peak, Some(seg("Noc 2", 2)));
}

// ---------- blocks ----------

#[test]
fn blocks_group_nodes_by_segment_in_time_order_with_loose_nodes_last() {
    let g = abbey();
    let s = sheet(&g, "petra");
    let blocks: Vec<(&str, Vec<&str>)> = s
        .blocks
        .iter()
        .map(|b| {
            (
                b.segment.as_str(),
                b.steps.iter().map(|st| st.id.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        blocks,
        [
            ("Příjezd", vec!["z-arrive"]),
            ("Noc", vec!["y-ghost"]),
            ("Ráno", vec!["w-verdict"]),
            ("", vec!["loose"]),
        ]
    );
}

#[test]
fn blocks_keep_several_nodes_of_one_segment_together() {
    let g = abbey();
    let s = sheet(&g, "crypt");
    assert_eq!(s.blocks.len(), 1);
    assert_eq!(s.blocks[0].segment, "Noc");
    let ids: Vec<&str> = s.blocks[0].steps.iter().map(|st| st.id.as_str()).collect();
    assert_eq!(ids, ["x-hide", "y-ghost"]);
}

#[test]
fn blocks_place_cycle_nodes_in_the_segment_of_their_step() {
    let g = cycled();
    let s = sheet(&g, "petra");
    let blocks: Vec<(&str, Vec<&str>)> = s
        .blocks
        .iter()
        .map(|b| {
            (
                b.segment.as_str(),
                b.steps.iter().map(|st| st.title.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        blocks,
        [
            ("Noc 1", vec!["Svět"]),
            ("Noc 2", vec!["Krok 1", "Krok 2"]),
            ("Noc 3", vec!["Krok 3"]),
        ]
    );
}

#[test]
fn when_is_set_only_for_cycle_nodes() {
    let g = cycled();
    let s = sheet(&g, "petra");
    assert_eq!(step(&s, "world").when, "");
    assert_eq!(step(&s, "c-step1").when, "Annin cyklus · 1. Ty");
    assert_eq!(step(&s, "a-step3").when, "Annin cyklus · 3. Odchod");
}

#[test]
fn steps_carry_title_text_and_decision_flag() {
    let g = abbey();
    let s = sheet(&g, "petra");
    let arrive = step(&s, "z-arrive");
    assert_eq!(arrive.title, "Příjezd hráčů");
    assert_eq!(arrive.text, "Přivítej hráče.");
    assert!(!arrive.decision);
    let verdict = step(&s, "w-verdict");
    assert!(verdict.decision);
    let labels: Vec<&str> = verdict.options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(labels, ["Obvinit", "Odejít"]);
}

// ---------- with ----------

#[test]
fn with_lists_the_other_owners_by_name_without_the_owner() {
    let g = abbey();
    let petra = sheet(&g, "petra");
    assert_eq!(step(&petra, "y-ghost").with, ["Bratr Řehoř", "Krypta"]);
    assert_eq!(step(&petra, "loose").with, [] as [String; 0]);
    let crypt = sheet(&g, "crypt");
    assert_eq!(step(&crypt, "y-ghost").with, ["Petra", "Bratr Řehoř"]);
}

// ---------- items ----------

#[test]
fn needs_from_names_the_owners_of_the_giving_nodes() {
    let g = abbey();
    let crypt = sheet(&g, "crypt");
    let lamp = find(&step(&crypt, "y-ghost").needs, "lamp");
    assert_eq!(lamp.name, "Lampa");
    assert_eq!(lamp.from, ["Petra"]);
}

#[test]
fn needs_from_names_the_starting_owners() {
    let g = abbey();
    let petra = sheet(&g, "petra");
    assert_eq!(
        find(&step(&petra, "y-ghost").needs, "clue").from,
        ["Bratr Řehoř"]
    );
    let crypt = sheet(&g, "crypt");
    let key = find(&step(&crypt, "x-hide").needs, "key");
    assert_eq!(key.from, ["Petra"]);
    assert_eq!(key.text, "Starý železný klíč.");
}

#[test]
fn needs_in_decision_options_name_every_giver() {
    let g = abbey();
    let petra = sheet(&g, "petra");
    let candle = find(&step(&petra, "w-verdict").options[0].needs, "candle");
    assert_eq!(
        candle.from,
        ["Petra", "Bratr Řehoř", "Krypta"],
        "given by `y-ghost` (three owners) and `z-arrive` (Petra again, not repeated)"
    );
}

#[test]
fn item_kind_is_info_for_info_and_item_for_items() {
    let g = abbey();
    let petra = sheet(&g, "petra");
    let needs = &step(&petra, "y-ghost").needs;
    assert_eq!(find(needs, "clue").kind, "info");
    assert_eq!(find(needs, "rumor").kind, "info");
    assert_eq!(find(needs, "lamp").kind, "item");
}

#[test]
fn gives_have_an_empty_from() {
    let g = abbey();
    let petra = sheet(&g, "petra");
    let gives = &step(&petra, "y-ghost").gives;
    assert_eq!(gives.len(), 1);
    assert_eq!(gives[0].name, "Svíčka");
    assert_eq!(gives[0].from, [] as [String; 0]);
    let option = &step(&petra, "w-verdict").options[0];
    assert!(option.gives.iter().all(|x| x.from.is_empty()));
    assert_eq!(option.gives[0].kind, "info");
}

// ---------- prepare ----------

#[test]
fn prepare_lists_starting_and_handed_out_items_deduplicated_by_name() {
    let g = abbey();
    let names: Vec<String> = sheet(&g, "petra")
        .prepare
        .into_iter()
        .map(|x| x.name)
        .collect();
    assert_eq!(
        names,
        ["Klíč", "Lampa", "Svíčka"],
        "candle is given twice; rumor and end are info"
    );
}

#[test]
fn prepare_is_empty_for_an_owner_that_hands_out_nothing() {
    let g = abbey();
    assert_eq!(sheet(&g, "gate").prepare, []);
}

#[test]
fn prepare_leaves_out_starting_info() {
    let g = abbey();
    let names: Vec<String> = sheet(&g, "mnich")
        .prepare
        .into_iter()
        .map(|x| x.name)
        .collect();
    assert_eq!(
        names,
        ["Svíčka"],
        "Řehoř starts with the clue (info) and hands out the candle in `y-ghost`"
    );
}

// ---------- identity ----------

#[test]
fn room_schedule_has_room_kind_no_role_and_the_room_color() {
    let g = abbey();
    let crypt = sheet(&g, "crypt");
    assert_eq!(crypt.name, "Krypta");
    assert_eq!(crypt.kind, "room");
    assert_eq!(crypt.role, None);
    assert_eq!(crypt.color, "#2c3e50");
    assert_eq!(sheet(&g, "gate").color, "");
}

#[test]
fn person_schedule_has_person_kind_role_and_color() {
    let g = abbey();
    let petra = sheet(&g, "petra");
    assert_eq!(
        (petra.kind, petra.role, petra.color.as_str()),
        ("person", Some("organizer"), "#c0392b")
    );
    let mnich = sheet(&g, "mnich");
    assert_eq!(
        (mnich.kind, mnich.role, mnich.color.as_str()),
        ("person", Some("npc"), "")
    );
    assert_eq!(mnich.name, "Bratr Řehoř");
}

#[test]
fn prepare_keeps_different_items_that_share_a_name_and_drops_repeats() {
    // `key-a` is a starting item and handed out again; `key-b` has the same name.
    let g = game(&[
        ("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n"),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"key-a\"\nname = \"Klíč\"\nowners = [\"petra\"]\nstarting = true\n\n\
             [[item]]\nid = \"key-b\"\nname = \"Klíč\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n",
        ),
        (
            "actions/a-give.toml",
            "[action]\ntitle = \"Dej\"\nowners = [\"petra\"]\ngives = [\"key-b\", \"key-a\"]\n",
        ),
        (
            "actions/b-end.toml",
            "[action]\ntitle = \"Konec\"\nneeds = [\"key-a\", \"key-b\"]\ngives = [\"end\"]\n",
        ),
    ]);
    let mut ids: Vec<String> = sheet(&g, "petra")
        .prepare
        .into_iter()
        .map(|x| x.id)
        .collect();
    ids.sort();
    assert_eq!(ids, ["key-a", "key-b"]);
}
