//! Property tests for escaping: `tex_escape` on arbitrary Unicode, and HTML escaping of user text
//! in a player sheet.

// Test-only crate: helper functions may fail loudly; clippy treats them as non-test code.
#![allow(clippy::unwrap_used)]

use std::fmt::Write as _;

use larp_model::{Game, Id, SourceFile, load};
use larp_render::{player_sheet_html, tex_escape};
use proptest::prelude::*;

/// The ten LaTeX special characters.
const TEX_SPECIAL: [char; 10] = ['\\', '{', '}', '$', '&', '#', '%', '_', '^', '~'];

/// Every escape sequence `tex_escape` documents, with the character it stands for.
const TEX_SEQUENCES: [(&str, char); 13] = [
    (r"\textbackslash{}", '\\'),
    (r"\textasciicircum{}", '^'),
    (r"\textasciitilde{}", '~'),
    (r"\textquotedbl{}", '"'),
    ("{[}", '['),
    ("{]}", ']'),
    (r"\{", '{'),
    (r"\}", '}'),
    (r"\$", '$'),
    (r"\&", '&'),
    (r"\#", '#'),
    (r"\%", '%'),
    (r"\_", '_'),
];

/// Characters likely to break escaping, Czech letters, control characters and anything else.
fn tricky_char() -> impl Strategy<Value = char> {
    prop_oneof![
        3 => prop::sample::select(vec![
            '\\', '{', '}', '$', '&', '#', '%', '_', '^', '~', '"', '[', ']', '<', '>', '\'', '/',
        ]),
        2 => prop::sample::select(vec![
            'ř', 'Ř', 'ů', 'Ů', 'č', 'Č', 'ě', 'š', 'ž', 'ý', 'á', 'í', 'é', 'ú', 'ň', 'ť', 'ď',
            '„', '“', '…', '–',
        ]),
        1 => prop::sample::select(vec![
            '\n', '\t', '\r', '\0', '\u{7}', '\u{8}', '\u{b}', '\u{c}', '\u{1b}', '\u{7f}',
            '\u{85}', '\u{9f}',
        ]),
        2 => prop::char::range('a', 'z'),
        1 => prop::char::range('0', '9'),
        2 => any::<char>(),
    ]
}

fn tricky_string() -> impl Strategy<Value = String> {
    prop::collection::vec(tricky_char(), 0..48).prop_map(|v| v.into_iter().collect())
}

/// Decodes the output of `tex_escape` back to text, failing on anything that is not a
/// documented escape sequence or a plain character.
fn tex_unescape(out: &str) -> Result<String, String> {
    let mut rest = out;
    let mut text = String::new();
    'outer: while let Some(c) = rest.chars().next() {
        for (seq, ch) in TEX_SEQUENCES {
            if let Some(after) = rest.strip_prefix(seq) {
                text.push(ch);
                rest = after;
                continue 'outer;
            }
        }
        if TEX_SPECIAL.contains(&c) || matches!(c, '"' | '[' | ']') {
            return Err(format!("unescaped {c:?} in {out:?}"));
        }
        text.push(c);
        rest = &rest[c.len_utf8()..];
    }
    Ok(text)
}

fn kept_by_tex(c: char) -> bool {
    !c.is_control() || c == '\n' || c == '\t'
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn tex_escape_leaves_no_unescaped_special_character(input in tricky_string()) {
        let out = tex_escape(&input);
        prop_assert!(tex_unescape(&out).is_ok(), "{:?}", tex_unescape(&out));
    }

    #[test]
    fn tex_escape_never_lets_a_backslash_command_through(input in tricky_string()) {
        let out = tex_escape(&input);
        // Every backslash starts one of the documented sequences, never a command from the input.
        let mut rest = out.as_str();
        while let Some(i) = rest.find('\\') {
            rest = &rest[i..];
            let known = TEX_SEQUENCES
                .iter()
                .find(|(seq, _)| seq.starts_with('\\') && rest.starts_with(seq));
            prop_assert!(known.is_some(), "stray backslash in {:?} (from {:?})", out, input);
            rest = &rest[known.unwrap().0.len()..];
        }
    }

    #[test]
    fn tex_escape_round_trips_everything_but_control_characters(input in tricky_string()) {
        let expected: String = input.chars().filter(|&c| kept_by_tex(c)).collect();
        prop_assert_eq!(tex_unescape(&tex_escape(&input)).unwrap(), expected);
    }

    #[test]
    fn tex_escape_drops_control_characters_except_newline_and_tab(input in tricky_string()) {
        let out = tex_escape(&input);
        prop_assert!(out.chars().all(kept_by_tex), "control character left in {:?}", out);
        let count = |s: &str, c: char| s.chars().filter(|&x| x == c).count();
        prop_assert_eq!(count(&out, '\n'), count(&input, '\n'));
        prop_assert_eq!(count(&out, '\t'), count(&input, '\t'));
    }

    #[test]
    fn tex_escape_keeps_letters_and_digits_unchanged(
        input in prop::collection::vec(
            prop_oneof![
                prop::char::range('a', 'z'),
                prop::char::range('A', 'Z'),
                prop::char::range('0', '9'),
                prop::sample::select(vec!['ř', 'Ř', 'ů', 'Ů', 'č', 'Č', 'ě', 'ž', 'ß', 'ж', '中']),
                any::<char>().prop_filter("alphanumeric", |c| c.is_alphanumeric()),
            ],
            0..48,
        ).prop_map(|v| v.into_iter().collect::<String>())
    ) {
        prop_assert_eq!(tex_escape(&input), input);
    }

    #[test]
    fn tex_escape_keeps_the_letters_and_digits_of_mixed_text_in_order(input in tricky_string()) {
        let alnum = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        let decoded = tex_unescape(&tex_escape(&input)).unwrap();
        prop_assert_eq!(alnum(&decoded), alnum(&input));
    }
}

// ---------- HTML ----------

/// Minijinja's HTML escaping, the documented auto-escape of the player sheet.
fn html_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '/' => out.push_str("&#x2f;"),
            _ => out.push(c),
        }
    }
    out
}

/// A TOML basic string holding `s`.
fn toml_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => write!(out, "\\u{:04X}", u32::from(c)).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One player whose name, goal, secret, backstory and decision brief carry `text` between
/// distinct markers.
fn game_with(text: &str) -> Game {
    let field = |marker: &str| toml_str(&format!("⟨{marker}·{text}·{marker}⟩"));
    let person = format!(
        "[person]\nname = {}\nrole = \"player\"\ngoals = [{}]\nsecrets = [{}]\n\n[person.sheet]\nbackstory = {}\n",
        field("jméno"),
        field("cíl"),
        field("tajemství"),
        field("příběh"),
    );
    let action = format!(
        "[decision]\ntitle = \"Řeč\"\nowners = [\"ann\"]\nbrief = {}\n\n[[decision.option]]\nlabel = \"Ano\"\n",
        field("brief"),
    );
    let files = [
        (
            "game.toml",
            "[game]\ntitle = \"Test\"\nlanguage = \"cs\"\n".to_owned(),
        ),
        ("people/ann.toml", person),
        ("actions/talk.toml", action),
    ];
    let files: Vec<SourceFile> = files
        .into_iter()
        .map(|(path, text)| SourceFile {
            path: path.to_owned(),
            text,
        })
        .collect();
    let checked = load(&files);
    assert!(checked.game.is_some(), "{:?}", checked.diagnostics);
    checked.game.unwrap()
}

fn html_sheet(text: &str) -> String {
    let g = game_with(text);
    let id = Id::parse("ann").unwrap();
    let graph = larp_graph::build(&g, &[], None);
    let destiny = larp_graph::destiny(&g, &graph, &id);
    player_sheet_html(&g, &id, &destiny, None).unwrap()
}

/// Text that HTML treats specially, mixed with Czech and plain characters (no control characters,
/// which TOML can carry but are irrelevant to HTML escaping).
fn html_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            3 => prop::sample::select(vec!['<', '>', '&', '"', '\'', '/', '=', ';', '#']),
            2 => prop::sample::select(vec!['ř', 'ů', 'č', 'Ž', '„', '“', ' ']),
            2 => prop::char::range('a', 'z'),
            1 => any::<char>().prop_filter("printable", |c| !c.is_control()),
        ],
        1..24,
    )
    .prop_map(|v| v.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn player_sheet_shows_user_text_only_html_escaped(text in html_text()) {
        let html = html_sheet(&text);
        for marker in ["jméno", "cíl", "tajemství", "příběh", "brief"] {
            let raw = format!("⟨{marker}·{text}·{marker}⟩");
            prop_assert!(
                html.contains(&html_escape(&raw)),
                "{} is missing in escaped form {:?}",
                marker,
                html_escape(&raw)
            );
            if raw != html_escape(&raw) {
                prop_assert!(!html.contains(&raw), "{} appears unescaped: {:?}", marker, raw);
            }
        }
    }

    #[test]
    fn player_sheet_contains_no_markup_from_user_text(text in html_text()) {
        let html = html_sheet(&text);
        // Between the markers there is never a raw `<`, `>`, `"` or `'`.
        for (start, _) in html.match_indices("⟨jméno·") {
            let rest = &html[start..];
            let end = rest.find("·jméno⟩").unwrap();
            let inner = &rest["⟨jméno·".len()..end];
            prop_assert!(!inner.contains(['<', '>', '"', '\'']), "raw markup in {:?}", inner);
        }
    }
}
