//! The packing list and handout cards (`packing_html`, `packing_tex`): escaping of game text,
//! empty-game messages, quantities, the location mark, template overrides and snapshots.
//!
//! Review snapshot changes with `cargo insta review`; accept them only when the new output is
//! intended.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use larp_graph::{Packing, packing};
use larp_model::{Game, SourceFile, load};
use larp_render::{Page, RenderError, packing_html, packing_tex};
use proptest::prelude::*;

fn load_game(files: &[(&str, &str)]) -> Game {
    let files: Vec<SourceFile> = files
        .iter()
        .map(|(p, t)| SourceFile {
            path: (*p).to_owned(),
            text: (*t).to_owned(),
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:#?}", checked.diagnostics);
    checked.game.unwrap()
}

/// One player, one room, three items and a handout info. Names, props and the handout contain
/// characters that HTML and LaTeX must escape, and Czech letters.
///
/// - `klic`: starting with Anna, 3 props;
/// - `kniha`: starting, lies in the cellar (before the game, so no `→` mark);
/// - `lampa`: its prop waits in the cellar, `prijezd` gives it (during, with `→ Sklep`);
/// - `dopis`: info with a handout, `prijezd` gives it;
/// - `pecet`: starting with the faction `rad`, whose member is Anna;
/// - `prsten`: starting, nobody holds it;
/// - `svice`: given by the option "Ano & {hned}" of `volba`.
fn game() -> Game {
    load_game(&[
        (
            "game.toml",
            "[game]\ntitle = \"Řád & Kříž\"\nlanguage = \"cs\"\n",
        ),
        (
            "people/anna.toml",
            "[person]\nname = \"Anna_#1 {Ř}\"\nrole = \"organizer\"\nfactions = [\"rad\"]\n",
        ),
        (
            "factions.toml",
            "[[faction]]\nid = \"rad\"\nname = \"Řád_{x}\"\n",
        ),
        (
            "rooms.toml",
            "[[room]]\nid = \"sklep\"\nname = \"Sklep <50 %>\"\n",
        ),
        (
            "items.toml",
            "[[item]]\nid = \"klic\"\nname = \"Klíč & $5\"\nowners = [\"anna\"]\nstarting = true\nprop = \"mosaz <stará> 100 %\"\nquantity = 3\n\n\
             [[item]]\nid = \"kniha\"\nname = \"Kniha ů\"\nowners = [\"anna\"]\nstarting = true\nlocation = \"sklep\"\n\n\
             [[item]]\nid = \"lampa\"\nname = \"Lampa_{x}\"\nprop = \"#2 petrolejka\"\nlocation = \"sklep\"\n\n\
             [[item]]\nid = \"pecet\"\nname = \"Pečeť\"\nowners = [\"rad\"]\nstarting = true\n\n\
             [[item]]\nid = \"prsten\"\nname = \"Prsten & co\"\nstarting = true\n\n\
             [[item]]\nid = \"svice\"\nname = \"Svíce\"\nprop = \"vosk\"\n",
        ),
        (
            "actions/volba.toml",
            "[decision]\ntitle = \"Volba\"\nowners = [\"anna\"]\n\n[[decision.option]]\nlabel = \"Ano & {hned}\"\ngives = [\"svice\"]\n\n[[decision.option]]\nlabel = \"Ne\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"dopis\"\nname = \"Dopis <tajný>\"\nhandout = \"\"\"\nExcelence & přátelé,\n50 % $ _ # { } <b>čeřů</b>\n\"\"\"\n",
        ),
        (
            "actions/prijezd.toml",
            "[action]\ntitle = \"Příjezd & {uvítání}\"\nowners = [\"anna\"]\nneeds = [\"klic\", \"kniha\"]\ngives = [\"lampa\", \"dopis\"]\n",
        ),
        (
            "actions/konec.toml",
            "[action]\ntitle = \"Konec\"\nowners = [\"anna\"]\nneeds = [\"lampa\", \"dopis\"]\n",
        ),
    ])
}

/// A game with nothing to pack and no handouts.
fn empty(language: &str) -> Game {
    load_game(&[(
        "game.toml",
        &format!("[game]\ntitle = \"Prázdno\"\nlanguage = \"{language}\"\n"),
    )])
}

fn html(g: &Game, page: Page) -> String {
    packing_html(g, &packing(g), page, None).unwrap()
}

fn tex(g: &Game, page: Page) -> String {
    packing_tex(g, &packing(g), page, None).unwrap()
}

// ---------- snapshots ----------

#[test]
fn packing_html_matches_snapshot() {
    insta::assert_snapshot!(html(&game(), Page::Packing));
}

#[test]
fn packing_tex_matches_snapshot() {
    insta::assert_snapshot!(tex(&game(), Page::Packing));
}

#[test]
fn handouts_html_matches_snapshot() {
    insta::assert_snapshot!(html(&game(), Page::Handouts));
}

#[test]
fn handouts_tex_matches_snapshot() {
    insta::assert_snapshot!(tex(&game(), Page::Handouts));
}

// ---------- HTML escaping ----------

#[test]
fn packing_html_escapes_names_props_rooms_and_titles() {
    let out = html(&game(), Page::Packing);
    for expected in [
        "Řád &amp; Kříž",
        "Klíč &amp; $5",
        "mosaz &lt;stará&gt; 100 %",
        "Sklep &lt;50 %&gt;",
        "Příjezd &amp; {uvítání}",
        "Anna_#1 {Ř}",
        "Dopis &lt;tajný&gt;",
        "Kniha ů",
    ] {
        assert!(out.contains(expected), "missing {expected:?} in:\n{out}");
    }
    for raw in ["<stará>", "<50 %>", "<tajný>", "Klíč & $5"] {
        assert!(!out.contains(raw), "unescaped {raw:?} in:\n{out}");
    }
}

#[test]
fn handouts_html_escapes_name_and_text() {
    let out = html(&game(), Page::Handouts);
    assert!(out.contains("Dopis &lt;tajný&gt;"), "{out}");
    assert!(out.contains("Excelence &amp; přátelé,"), "{out}");
    assert!(
        out.contains("50 % $ _ # { } &lt;b&gt;čeřů&lt;&#x2f;b&gt;")
            || out.contains("50 % $ _ # { } &lt;b&gt;čeřů&lt;/b&gt;"),
        "{out}"
    );
    assert!(!out.contains("<b>"), "{out}");
}

// ---------- LaTeX escaping ----------

#[test]
fn packing_tex_escapes_names_props_rooms_and_titles() {
    let out = tex(&game(), Page::Packing);
    for expected in [
        r"Řád \& Kříž",
        r"Klíč \& \$5",
        r"mosaz <stará> 100 \%",
        r"Sklep <50 \%>",
        r"Příjezd \& \{uvítání\}",
        r"Anna\_\#1 \{Ř\}",
        r"Lampa\_\{x\}",
        r"\#2 petrolejka",
    ] {
        assert!(out.contains(expected), "missing {expected:?} in:\n{out}");
    }
    for raw in ["Klíč & $5", "100 %", "Lampa_{x}", "{uvítání}"] {
        assert!(!out.contains(raw), "unescaped {raw:?} in:\n{out}");
    }
}

#[test]
fn handouts_tex_escapes_name_and_text() {
    let out = tex(&game(), Page::Handouts);
    assert!(out.contains(r"Dopis <tajný>"), "{out}");
    assert!(out.contains(r"Excelence \& přátelé,"), "{out}");
    assert!(out.contains(r"50 \% \$ \_ \# \{ \} <b>čeřů</b>"), "{out}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Whatever the handout says, the LaTeX card holds it escaped and the HTML card never holds
    /// a raw `<` or `&` from it.
    #[test]
    fn handout_text_is_always_escaped(text in "[a-zčřůŘ <>&%$_#{}\n]{1,30}") {
        let p = Packing {
            handouts: vec![("Karta".to_owned(), text.clone())],
            ..Packing::default()
        };
        let g = empty("cs");
        let html = packing_html(&g, &p, Page::Handouts, Some("{{ p.handouts[0][1] }}")).unwrap();
        prop_assert!(!html.contains('<') && !html.contains("& "), "{:?}", html);
        let tex = packing_tex(&g, &p, Page::Handouts, Some("{{ p.handouts[0][1] }}")).unwrap();
        prop_assert_eq!(tex, larp_render::tex_escape(&text));
    }
}

// ---------- quantity and location ----------

#[test]
fn packing_html_shows_a_quantity_above_one() {
    let out = html(&game(), Page::Packing);
    assert!(out.contains("3× <strong>Klíč"), "{out}");
    assert!(!out.contains("1×"), "{out}");
}

#[test]
fn packing_tex_shows_a_quantity_above_one() {
    let out = tex(&game(), Page::Packing);
    assert!(out.contains(r"3× {\bfseries Klíč"), "{out}");
    assert!(!out.contains("1×"), "{out}");
}

#[test]
fn packing_html_marks_the_location_only_during_the_game() {
    let out = html(&game(), Page::Packing);
    // `kniha` lies in the cellar before the game; only `lampa`, handed out during it, gets a mark.
    assert_eq!(out.matches("→ ").count(), 1, "{out}");
    assert!(out.contains("→ Sklep &lt;50 %&gt;"), "{out}");
    let before = &out[..out.find("Během hry").unwrap()];
    assert!(!before.contains('→'), "{before}");
}

#[test]
fn packing_tex_marks_the_location_only_during_the_game() {
    let out = tex(&game(), Page::Packing);
    assert_eq!(out.matches(r"$\rightarrow$").count(), 1, "{out}");
    assert!(out.contains(r"$\rightarrow$ Sklep <50 \%>"), "{out}");
}

#[test]
fn packing_marks_a_handout_card() {
    assert!(html(&game(), Page::Packing).contains(">kartička</span>"));
    assert!(tex(&game(), Page::Packing).contains("(kartička)"));
}

// ---------- empty games ----------

#[test]
fn packing_of_an_empty_game_says_so() {
    let g = empty("cs");
    let msg = "Hra nemá žádné předměty ani kartičky.";
    assert!(html(&g, Page::Packing).contains(msg));
    assert!(tex(&g, Page::Packing).contains(msg));
}

#[test]
fn handouts_of_a_game_without_handouts_say_so() {
    let g = empty("cs");
    let msg = "Žádná informace ani předmět nemá kartičku (handout).";
    assert!(html(&g, Page::Handouts).contains(msg));
    assert!(tex(&g, Page::Handouts).contains(msg));
}

#[test]
fn empty_messages_follow_the_game_language() {
    let g = empty("en");
    assert!(html(&g, Page::Packing).contains("The game has no items or handouts."));
    assert!(html(&g, Page::Handouts).contains("No item or info has a handout."));
    assert!(tex(&g, Page::Packing).contains("The game has no items or handouts."));
}

#[test]
fn packing_with_things_has_no_empty_message() {
    let g = game();
    assert!(!html(&g, Page::Packing).contains("Hra nemá"));
    assert!(!tex(&g, Page::Packing).contains("Hra nemá"));
    assert!(!html(&g, Page::Handouts).contains("Žádná informace"));
    assert!(!tex(&g, Page::Handouts).contains("Žádná informace"));
}

#[test]
fn packing_with_only_handout_cards_during_the_game_has_no_empty_message() {
    // Nothing starts anywhere; one act hands out a card.
    let g = load_game(&[
        ("game.toml", "[game]\ntitle = \"T\"\nlanguage = \"cs\"\n"),
        (
            "people/anna.toml",
            "[person]\nname = \"Anna\"\nrole = \"organizer\"\n",
        ),
        (
            "info.toml",
            "[[info]]\nid = \"karta\"\nname = \"Karta\"\nhandout = \"Text\"\nending = true\n",
        ),
        (
            "actions/a.toml",
            "[action]\ntitle = \"A\"\nowners = [\"anna\"]\ngives = [\"karta\"]\n",
        ),
    ]);
    let out = html(&g, Page::Packing);
    assert!(!out.contains("Hra nemá"), "{out}");
    assert!(out.contains("Během hry"), "{out}");
    assert!(!out.contains("Před hrou"), "{out}");
}

// ---------- pages and templates ----------

#[test]
fn page_chooses_the_built_in_template() {
    let g = game();
    assert!(html(&g, Page::Packing).contains("<title>Balicí seznam · Řád &amp; Kříž</title>"));
    assert!(
        html(&g, Page::Handouts).contains("<title>Kartičky pro hráče · Řád &amp; Kříž</title>")
    );
    assert!(tex(&g, Page::Packing).contains(r"\packpart{Před hrou}"));
    assert!(tex(&g, Page::Handouts).contains(r"\handout{Dopis <tajný>}"));
}

#[test]
fn custom_html_template_replaces_the_built_in_one_and_still_escapes() {
    let g = game();
    let p = packing(&g);
    let tpl = "CUSTOM {{ game_title }} {{ p.handouts | length }} {{ t.pack_title }}";
    for page in [Page::Packing, Page::Handouts] {
        assert_eq!(
            packing_html(&g, &p, page, Some(tpl)).unwrap(),
            "CUSTOM Řád &amp; Kříž 1 Balicí seznam"
        );
    }
}

#[test]
fn custom_tex_template_replaces_the_built_in_one_and_still_escapes() {
    let g = game();
    let p = packing(&g);
    let tpl = "CUSTOM {{ game_title }} {% for x in p.before %}{{ x.name }};{% endfor %}";
    for page in [Page::Packing, Page::Handouts] {
        assert_eq!(
            packing_tex(&g, &p, page, Some(tpl)).unwrap(),
            r"CUSTOM Řád \& Kříž Anna\_\#1 \{Ř\};Sklep <50 \%>;Řád\_\{x\};;"
        );
    }
}

#[test]
fn invalid_template_is_a_template_error() {
    let g = game();
    let p = packing(&g);
    assert!(matches!(
        packing_html(&g, &p, Page::Packing, Some("{% for %}")),
        Err(RenderError::Template(_))
    ));
    assert!(matches!(
        packing_tex(&g, &p, Page::Handouts, Some("{{ p.")),
        Err(RenderError::Template(_))
    ));
}

#[test]
fn packing_pages_are_deterministic() {
    let render = || {
        let g = game();
        [
            html(&g, Page::Packing),
            tex(&g, Page::Packing),
            html(&g, Page::Handouts),
            tex(&g, Page::Handouts),
        ]
    };
    let first = render();
    for _ in 0..5 {
        assert_eq!(render(), first);
    }
}

// ---------- members, unassigned, options, waiting props, line breaks ----------

#[test]
fn packing_names_unassigned_things_and_faction_members() {
    let out = html(&game(), Page::Packing);
    assert!(out.contains("<h3>Nikdo to nemá</h3>"), "{out}");
    assert!(out.contains("Prsten &amp; co"), "{out}");
    assert!(out.contains("členové: Anna_#1 {Ř}"), "{out}");
    let out = tex(&game(), Page::Packing);
    assert!(out.contains(r"\packplace{Nikdo to nemá}"), "{out}");
    assert!(out.contains(r"členové: Anna\_\#1 \{Ř\}"), "{out}");
}

#[test]
fn packing_shows_the_option_that_hands_a_thing_out() {
    let out = html(&game(), Page::Packing);
    assert!(out.contains("(při volbě „Ano &amp; {hned}“)"), "{out}");
    let out = tex(&game(), Page::Packing);
    assert!(out.contains(r"(při volbě „Ano \& \{hned\}“)"), "{out}");
}

#[test]
fn packing_shows_which_acts_a_waiting_prop_is_for() {
    let out = html(&game(), Page::Packing);
    assert!(out.contains("(pro Příjezd &amp; {uvítání})"), "{out}");
    let out = tex(&game(), Page::Packing);
    assert!(out.contains(r"(pro Příjezd \& \{uvítání\})"), "{out}");
}

#[test]
fn english_quotes_follow_the_game_language() {
    let mut g = game();
    g.meta.language = larp_model::model::Language::En;
    let out = html(&g, Page::Packing);
    assert!(out.contains("(if “Ano &amp; {hned}”)"), "{out}");
}

#[test]
fn handouts_tex_keeps_line_breaks_of_the_handout() {
    let out = tex(&game(), Page::Handouts);
    assert!(
        out.contains("Excelence \\& přátelé,\\newline\n50 \\%"),
        "{out}"
    );
}

#[test]
fn tex_lines_keeps_single_breaks_and_paragraphs() {
    assert_eq!(
        larp_render::tex_lines("Řádek & 1\nŘádek 2\n\nNový odstavec_"),
        "Řádek \\& 1\\newline\nŘádek 2\n\nNový odstavec\\_"
    );
}

#[test]
fn tex_lines_trims_surrounding_blank_lines() {
    assert_eq!(larp_render::tex_lines("\n\nčára\n\n"), "čára");
}

#[test]
fn tex_lines_of_empty_text_is_empty() {
    assert_eq!(larp_render::tex_lines(""), "");
}

#[test]
fn tex_lines_of_one_line_is_tex_escape() {
    assert_eq!(larp_render::tex_lines("50 % ů"), "50 \\% ů");
}

#[test]
fn tex_lines_never_starts_a_paragraph_with_a_line_break() {
    // Two blank lines in a row, or a blank line with spaces, must not leave `\newline` at the
    // start of a paragraph: LaTeX stops with "There's no line here to end".
    for text in ["a\n\n\nb", "a\n\n\n\nb", "a\n\n \nb", "a\n \n\nb"] {
        let out = larp_render::tex_lines(text);
        for para in out.split("\n\n") {
            assert!(
                !para.trim_start().starts_with(r"\newline"),
                "{text:?} -> {out:?}"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn tex_lines_paragraphs_never_start_or_end_with_a_line_break(
        text in "[a-zř%& \n]{0,40}",
    ) {
        let out = larp_render::tex_lines(&text);
        for para in out.split("\n\n") {
            prop_assert!(!para.trim_start().starts_with(r"\newline"), "{:?} -> {:?}", text, out);
            prop_assert!(!para.trim_end().ends_with(r"\newline"), "{:?} -> {:?}", text, out);
        }
    }
}
