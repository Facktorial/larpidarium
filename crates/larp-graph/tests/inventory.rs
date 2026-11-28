//! The Inventory tab and the packing list (`inventory`, `packing`) from small in-memory games.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used, clippy::panic)]

use larp_graph::{InventoryItem, Move, PackItem, Packing, PackingPlace, inventory, packing};
use larp_model::{Game, SourceFile, load};

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

const GAME: &str = "[game]\ntitle = \"Opatství\"\nlanguage = \"cs\"\n";

/// Two segments; Petra (organizer), Marta (player), Anselm (NPC); the faction Konvent (Marta and
/// Anselm); rooms Knihovna and Krypta.
///
/// Items, in file order: `key` (Anselm, starting, 2 props, category), `bones` (starting, in the
/// crypt), `lamp` (prop waits in the library, given by `z-arrive`), `candle` (given by both
/// options of `w-choice`), `seal` (starting, Marta and Anselm, 3 props each), `book` (Marta, starting, but lies
/// in the library, handout), `coin` (given by the untimed `x-loose`).
/// Info: `rumor` (plain, given by `z-arrive`), `letter` (handout, given by `z-arrive`),
/// `card` (prop, starting with the faction), `end`.
///
/// File names are chosen so alphabetical order differs from time order: `z-arrive` (Příjezd),
/// `w-choice` and `y-crypt` (Noc), `x-loose` (no time).
fn abbey() -> Game {
    game(&[
        ("game.toml", GAME),
        (
            "segments.toml",
            "[[segment]]\nid = \"one\"\nname = \"Příjezd\"\n\n[[segment]]\nid = \"two\"\nname = \"Noc\"\n",
        ),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "people/marta.toml",
            "[person]\nname = \"Marta Řeháková\"\nrole = \"player\"\nfactions = [\"bratri\"]\n",
        ),
        (
            "people/anselm.toml",
            "[person]\nname = \"Bratr Anselm\"\nrole = \"npc\"\nfactions = [\"bratri\"]\n",
        ),
        (
            "factions.toml",
            "[[faction]]\nid = \"bratri\"\nname = \"Konvent\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"library\"\nname = \"Knihovna\"\n\n[[room]]\nid = \"crypt\"\nname = \"Krypta\"\n",
        ),
        (
            "items.toml",
            "[[category]]\nid = \"klic\"\n\n\
             [[item]]\nid = \"key\"\nname = \"Klíč\"\ncategory = \"klic\"\nowners = [\"anselm\"]\nstarting = true\nprop = \"železný klíč\"\nquantity = 2\n\n\
             [[item]]\nid = \"bones\"\nname = \"Kosti\"\nowners = [\"crypt\"]\nstarting = true\nprop = \"kostra\"\nlocation = \"crypt\"\n\n\
             [[item]]\nid = \"lamp\"\nname = \"Lampa\"\nprop = \"petrolejka\"\nlocation = \"library\"\n\n\
             [[item]]\nid = \"candle\"\nname = \"Svíčka\"\n\n\
             [[item]]\nid = \"seal\"\nname = \"Pečeť\"\nowners = [\"marta\", \"anselm\"]\nstarting = true\nquantity = 3\n\n\
             [[item]]\nid = \"book\"\nname = \"Kniha\"\nowners = [\"marta\"]\nstarting = true\nlocation = \"library\"\nhandout = \"Strana 13.\"\n\n\
             [[item]]\nid = \"coin\"\nname = \"Mince\"\nprop = \"stříbrňák\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"rumor\"\nname = \"Fáma\"\n\n\
             [[info]]\nid = \"letter\"\nname = \"Dopis\"\nhandout = \"Excelence, píši Vám v tísni.\"\n\n\
             [[info]]\nid = \"card\"\nname = \"Pečetní karta\"\nowners = [\"bratri\"]\nstarting = true\nprop = \"kartička\"\n\n\
             [[info]]\nid = \"end\"\nname = \"Konec\"\nending = true\n",
        ),
        (
            "actions/z-arrive.toml",
            "[action]\ntitle = \"Příjezd\"\nsegment = \"one\"\nowners = [\"petra\"]\ngives = [\"lamp\", \"rumor\", \"letter\"]\n",
        ),
        (
            "actions/y-crypt.toml",
            "[action]\ntitle = \"Noc v kryptě\"\nsegment = \"two\"\nowners = [\"anselm\", \"crypt\"]\nneeds = [\"lamp\", \"key\", \"rumor\", \"letter\"]\nuses = [\"bones\"]\n",
        ),
        (
            "actions/w-choice.toml",
            "[decision]\ntitle = \"Volba\"\nsegment = \"two\"\nowners = [\"marta\", \"bratri\"]\n\n\
             [[decision.option]]\nlabel = \"Zapálit\"\nuses = [\"lamp\"]\ngives = [\"candle\"]\n\n\
             [[decision.option]]\nlabel = \"Odejít\"\nneeds = [\"seal\"]\ngives = [\"candle\", \"end\"]\n",
        ),
        (
            "actions/x-loose.toml",
            "[action]\ntitle = \"Kdykoli\"\nowners = [\"petra\"]\ngives = [\"coin\"]\n",
        ),
    ])
}

fn find<'a>(inv: &'a [InventoryItem], id: &str) -> &'a InventoryItem {
    inv.iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("{id} is not in the inventory"))
}

fn mv(node: &str, title: &str, option: Option<&str>, owners: &[&str]) -> Move {
    Move {
        node: node.to_owned(),
        title: title.to_owned(),
        option: option.map(str::to_owned),
        owners: owners.iter().map(|o| (*o).to_owned()).collect(),
    }
}

fn names(items: &[PackItem]) -> Vec<&str> {
    items.iter().map(|i| i.name.as_str()).collect()
}

// ---------- inventory ----------

#[test]
fn inventory_lists_items_in_file_order_then_info_with_a_prop_or_handout() {
    let inv = inventory(&abbey());
    let ids: Vec<(&str, &str)> = inv.iter().map(|i| (i.id.as_str(), i.kind)).collect();
    assert_eq!(
        ids,
        [
            ("key", "item"),
            ("bones", "item"),
            ("lamp", "item"),
            ("candle", "item"),
            ("seal", "item"),
            ("book", "item"),
            ("coin", "item"),
            ("letter", "info"),
            ("card", "info"),
        ]
    );
}

#[test]
fn inventory_leaves_out_info_without_prop_or_handout() {
    let inv = inventory(&abbey());
    assert!(inv.iter().all(|i| i.id != "rumor" && i.id != "end"));
}

#[test]
fn inventory_shows_category_prop_and_quantity() {
    let inv = inventory(&abbey());
    let key = find(&inv, "key");
    assert_eq!(key.name, "Klíč");
    assert_eq!(key.category, "klic");
    assert_eq!(key.prop, "železný klíč");
    assert_eq!(key.quantity, 2);
}

#[test]
fn inventory_leaves_category_and_prop_empty_and_quantity_one_by_default() {
    let inv = inventory(&abbey());
    let candle = find(&inv, "candle");
    assert_eq!(candle.category, "");
    assert_eq!(candle.prop, "");
    assert_eq!(candle.quantity, 1);
    assert_eq!(candle.location, "");
    assert_eq!(candle.handout, "");
}

#[test]
fn inventory_shows_the_location_as_the_room_name() {
    let inv = inventory(&abbey());
    assert_eq!(find(&inv, "lamp").location, "Knihovna");
    assert_eq!(find(&inv, "bones").location, "Krypta");
}

#[test]
fn inventory_names_starting_holders_people_rooms_and_factions() {
    let inv = inventory(&abbey());
    assert_eq!(
        find(&inv, "seal").holders,
        ["Marta Řeháková", "Bratr Anselm"]
    );
    assert_eq!(find(&inv, "bones").holders, ["Krypta"]);
    assert_eq!(find(&inv, "card").holders, ["Konvent"]);
}

#[test]
fn inventory_has_no_holders_for_things_that_are_not_starting() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/marta.toml",
            "[person]\nname = \"Marta\"\nrole = \"player\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"coin\"\nname = \"Mince\"\nowners = [\"marta\"]\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"marta\"]\ngives = [\"coin\"]\n",
        ),
    ]);
    assert_eq!(inventory(&g)[0].holders, [] as [String; 0]);
}

#[test]
fn inventory_shows_the_handout_text() {
    let inv = inventory(&abbey());
    assert_eq!(find(&inv, "letter").handout, "Excelence, píši Vám v tísni.");
    assert_eq!(find(&inv, "book").handout, "Strana 13.");
}

#[test]
fn inventory_lists_the_acts_that_give_need_and_use_a_thing_with_owner_names() {
    let inv = inventory(&abbey());
    let lamp = find(&inv, "lamp");
    assert_eq!(lamp.given, [mv("z-arrive", "Příjezd", None, &["Petra"])]);
    assert_eq!(
        lamp.needed,
        [mv(
            "y-crypt",
            "Noc v kryptě",
            None,
            &["Bratr Anselm", "Krypta"]
        )]
    );
    assert_eq!(
        lamp.used,
        [mv(
            "w-choice",
            "Volba",
            Some("Zapálit"),
            &["Marta Řeháková", "Konvent"]
        )]
    );
}

#[test]
fn inventory_lists_each_decision_option_that_gives_a_thing() {
    let inv = inventory(&abbey());
    let owners = ["Marta Řeháková", "Konvent"];
    let candle = find(&inv, "candle");
    assert_eq!(
        candle.given,
        [
            mv("w-choice", "Volba", Some("Zapálit"), &owners),
            mv("w-choice", "Volba", Some("Odejít"), &owners),
        ]
    );
    assert!(candle.needed.is_empty() && candle.used.is_empty());
    assert_eq!(
        find(&inv, "seal").needed,
        [mv("w-choice", "Volba", Some("Odejít"), &owners)]
    );
}

#[test]
fn inventory_moves_are_empty_for_a_thing_no_act_touches() {
    let inv = inventory(&abbey());
    let card = find(&inv, "card");
    assert!(card.given.is_empty() && card.needed.is_empty() && card.used.is_empty());
}

#[test]
fn inventory_of_a_game_without_items_is_empty() {
    assert_eq!(inventory(&game(&[("game.toml", GAME)])), []);
}

#[test]
fn graph_data_carries_the_inventory() {
    let g = abbey();
    let data = larp_graph::build(&g, &[], None);
    assert_eq!(data.inventory, inventory(&g));
}

// ---------- packing: before the game ----------

fn place<'a>(p: &'a Packing, name: &str) -> &'a PackingPlace {
    p.before
        .iter()
        .find(|pl| pl.name == name)
        .unwrap_or_else(|| panic!("no place {name:?} in {:?}", p.before))
}

#[test]
fn packing_before_groups_things_by_room_or_holder_sorted_by_place() {
    let p = packing(&abbey());
    let places: Vec<(&str, Vec<&str>)> = p
        .before
        .iter()
        .map(|pl| (pl.name.as_str(), names(&pl.items)))
        .collect();
    assert_eq!(
        places,
        [
            ("Bratr Anselm", vec!["Klíč", "Pečeť"]),
            ("Knihovna", vec!["Kniha", "Lampa"]),
            ("Konvent", vec!["Pečetní karta"]),
            ("Krypta", vec!["Kosti"]),
            ("Marta Řeháková", vec!["Pečeť"]),
        ]
    );
}

#[test]
fn packing_before_puts_a_starting_thing_with_a_location_only_in_its_room() {
    let p = packing(&abbey());
    assert!(!names(&place(&p, "Marta Řeháková").items).contains(&"Kniha"));
    let book = &place(&p, "Knihovna").items[0];
    assert_eq!(book.name, "Kniha");
    assert!(book.handout);
    assert_eq!(book.for_acts, [] as [String; 0]);
}

#[test]
fn packing_before_puts_a_waiting_prop_in_its_room_with_the_acts_that_give_it() {
    let p = packing(&abbey());
    let lamp = &place(&p, "Knihovna").items[1];
    assert_eq!(
        *lamp,
        PackItem {
            name: "Lampa".to_owned(),
            prop: "petrolejka".to_owned(),
            quantity: 1,
            handout: false,
            location: String::new(),
            option: None,
            for_acts: vec!["Příjezd".to_owned()],
        }
    );
}

#[test]
fn packing_before_leaves_out_things_that_are_not_starting_and_have_no_location() {
    let p = packing(&abbey());
    let all: Vec<&str> = p.before.iter().flat_map(|pl| names(&pl.items)).collect();
    for absent in ["Svíčka", "Mince", "Dopis", "Fáma"] {
        assert!(!all.contains(&absent), "{absent} in {all:?}");
    }
}

#[test]
fn packing_before_counts_quantity_per_holder() {
    let p = packing(&abbey());
    let seal = |name| {
        place(&p, name)
            .items
            .iter()
            .find(|i| i.name == "Pečeť")
            .unwrap()
            .quantity
    };
    assert_eq!((seal("Bratr Anselm"), seal("Marta Řeháková")), (3, 3));
    assert_eq!(place(&p, "Bratr Anselm").items[0].quantity, 2);
}

#[test]
fn packing_before_names_the_members_of_a_faction_only() {
    let p = packing(&abbey());
    assert_eq!(
        place(&p, "Konvent").members,
        ["Bratr Anselm", "Marta Řeháková"]
    );
    assert_eq!(place(&p, "Krypta").members, [] as [String; 0]);
    assert_eq!(place(&p, "Bratr Anselm").members, [] as [String; 0]);
}

#[test]
fn packing_before_sorts_items_within_a_place_by_name() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/marta.toml",
            "[person]\nname = \"Marta\"\nrole = \"player\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"c\"\nname = \"Čaj\"\nowners = [\"marta\"]\nstarting = true\n\n\
             [[item]]\nid = \"b\"\nname = \"Bota\"\nowners = [\"marta\"]\nstarting = true\n\n\
             [[item]]\nid = \"a\"\nname = \"Zvon\"\nowners = [\"marta\"]\nstarting = true\n\n\
             [[item]]\nid = \"d\"\nname = \"Aha\"\nowners = [\"marta\"]\nstarting = true\n",
        ),
    ]);
    let p = packing(&g);
    assert_eq!(p.before.len(), 1);
    // Byte order: Č (U+010C) sorts after ASCII letters.
    assert_eq!(names(&p.before[0].items), ["Aha", "Bota", "Zvon", "Čaj"]);
}

/// Petra and a room both named "Petra"; a starting item with no owner and no location; one
/// starting item each for Petra and the room.
fn same_names() -> Game {
    game(&[
        ("game.toml", GAME),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"petra-room\"\nname = \"Petra\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"stray\"\nname = \"Ztracený prsten\"\nstarting = true\nprop = \"prsten\"\n\n\
             [[item]]\nid = \"a\"\nname = \"Ampule\"\nowners = [\"petra\"]\nstarting = true\n\n\
             [[item]]\nid = \"b\"\nname = \"Bedna\"\nstarting = true\nlocation = \"petra-room\"\n",
        ),
    ])
}

#[test]
fn packing_before_puts_unheld_starting_things_last_under_an_empty_name() {
    let p = packing(&same_names());
    let last = p.before.last().unwrap();
    assert_eq!(last.name, "");
    assert_eq!(names(&last.items), ["Ztracený prsten"]);
    assert_eq!(last.members, [] as [String; 0]);
}

#[test]
fn packing_before_keeps_a_room_and_a_person_with_the_same_name_apart() {
    let p = packing(&same_names());
    let petras: Vec<Vec<&str>> = p
        .before
        .iter()
        .filter(|pl| pl.name == "Petra")
        .map(|pl| names(&pl.items))
        .collect();
    assert_eq!(petras.len(), 2, "{:?}", p.before);
    assert!(petras.contains(&vec!["Ampule"]) && petras.contains(&vec!["Bedna"]));
}

// ---------- packing: during the game ----------

#[test]
fn packing_during_lists_giving_acts_in_time_order_with_untimed_last() {
    let p = packing(&abbey());
    let titles: Vec<&str> = p.during.iter().map(|a| a.title.as_str()).collect();
    // `y-crypt` gives nothing, so it is not listed.
    assert_eq!(titles, ["Příjezd", "Volba", "Kdykoli"]);
}

#[test]
fn packing_during_orders_untimed_acts_by_id() {
    let g = game(&[
        ("game.toml", GAME),
        (
            "items.toml",
            "[[item]]\nid = \"x\"\nname = \"X\"\n\n[[item]]\nid = \"y\"\nname = \"Y\"\n",
        ),
        (
            "actions/b.toml",
            "[action]\ntitle = \"Druhá\"\ngives = [\"x\"]\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"První\"\ngives = [\"y\"]\n",
        ),
    ]);
    let titles: Vec<String> = packing(&g).during.into_iter().map(|a| a.title).collect();
    assert_eq!(titles, ["První", "Druhá"]);
}

#[test]
fn packing_during_names_the_owners_of_each_act() {
    let p = packing(&abbey());
    assert_eq!(p.during[0].owners, ["Petra"]);
    assert_eq!(p.during[1].owners, ["Marta Řeháková", "Konvent"]);
}

#[test]
fn packing_during_lists_items_and_handout_info_but_not_plain_info() {
    let p = packing(&abbey());
    let arrive = &p.during[0];
    assert_eq!(names(&arrive.items), ["Lampa", "Dopis"]);
    assert_eq!(arrive.items[0].location, "Knihovna");
    assert_eq!(arrive.items[0].prop, "petrolejka");
    assert_eq!(arrive.items[0].option, None);
    assert_eq!(arrive.items[0].for_acts, [] as [String; 0]);
    assert!(arrive.items[1].handout);
}

#[test]
fn packing_during_lists_a_thing_once_per_option_that_gives_it() {
    let p = packing(&abbey());
    let choice: Vec<(&str, Option<&str>)> = p.during[1]
        .items
        .iter()
        .map(|i| (i.name.as_str(), i.option.as_deref()))
        .collect();
    // `end` is a plain info, so only the candle is listed.
    assert_eq!(
        choice,
        [("Svíčka", Some("Zapálit")), ("Svíčka", Some("Odejít"))]
    );
}

#[test]
fn packing_during_lists_two_things_with_the_same_name_given_by_one_act() {
    // The letter as an item and its content as an info, both named "Dopis".
    let g = game(&[
        ("game.toml", GAME),
        (
            "people/petra.toml",
            "[person]\nname = \"Petra\"\nrole = \"organizer\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"letter\"\nname = \"Dopis\"\nprop = \"obálka\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"letter-text\"\nname = \"Dopis\"\nhandout = \"Text dopisu\"\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"Předání\"\nowners = [\"petra\"]\ngives = [\"letter\", \"letter-text\"]\n",
        ),
    ]);
    let p = packing(&g);
    let items = &p.during[0].items;
    assert_eq!(items.len(), 2, "{items:?}");
    assert!(!items[0].handout && items[1].handout);
}

// ---------- packing: handouts ----------

#[test]
fn packing_handouts_are_name_and_text_of_items_then_info() {
    let p = packing(&abbey());
    assert_eq!(
        p.handouts,
        [
            ("Kniha".to_owned(), "Strana 13.".to_owned()),
            (
                "Dopis".to_owned(),
                "Excelence, píši Vám v tísni.".to_owned()
            ),
        ]
    );
}

#[test]
fn packing_of_a_game_without_items_is_empty() {
    assert_eq!(packing(&game(&[("game.toml", GAME)])), Packing::default());
}
