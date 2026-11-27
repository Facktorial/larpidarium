//! `larpidarium schema`: JSON Schemas of the game files, for autocomplete and checks in editors
//! (Taplo / "Even Better TOML" in VS Code, and other editors with TOML language servers).
//!
//! The schemas mirror `larp_model::model`; a test checks that every key used in the example games
//! is described here, and `deny_unknown_fields` in the model rejects anything else.

use std::fs;
use std::path::Path;

use miette::{IntoDiagnostic, Result, WrapErr, miette};
use serde_json::{Value, json};

/// First line of a `.taplo.toml` written by this command; such a file may be overwritten.
const MARKER: &str = "# Written by `larpidarium schema`";

/// One json: file name stem and the game-folder globs it applies to.
pub(crate) struct Schema {
    /// `<name>.schema.json`.
    pub(crate) name: &'static str,
    /// Paths inside the game folder, as Taplo globs.
    pub(crate) globs: &'static [&'static str],
    /// The JSON Schema.
    pub(crate) json: Value,
}

fn id(what: &str) -> Value {
    json!({ "type": "string", "pattern": "^[a-z][a-z0-9]*(-[a-z0-9]+)*$",
            "description": format!("Id of {what}: lowercase ASCII letters, digits and dashes.") })
}
fn reference(what: &str) -> Value {
    json!({ "type": "string", "pattern": "^[a-z][a-z0-9]*(-[a-z0-9]+)*$",
            "description": format!("Id of {what}.") })
}
fn refs(what: &str) -> Value {
    json!({ "type": "array", "items": reference(what), "description": format!("Ids of {what}.") })
}
fn text(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}
fn texts(description: &str) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": description })
}
fn color() -> Value {
    json!({ "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "description": "Color as #rrggbb." })
}
fn step() -> Value {
    json!({ "type": "integer", "minimum": 1, "maximum": 255,
            "description": "Step of the cycle, 1 … number of steps in game.toml." })
}
fn object(required: &[&str], properties: &Value) -> Value {
    json!({ "type": "object", "additionalProperties": false, "required": required,
            "properties": properties })
}
fn list_of(item: &Value, description: &str) -> Value {
    json!({ "type": "array", "items": item, "description": description })
}
fn file(title: &str, required: &[&str], properties: &Value) -> Value {
    let mut v = object(required, properties);
    v["$schema"] = json!("http://json-schema.org/draft-07/schema#");
    v["title"] = json!(title);
    v
}

fn flow() -> Value {
    json!({
        "needs": refs("items or info that must exist; they stay"),
        "uses": refs("items that must exist and are used up"),
        "gives": refs("items or info produced"),
    })
}

fn node_common() -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("title".into(), text("Short title shown on the graph."));
    m.insert("owners".into(), refs("people or rooms that own it"));
    m.insert(
        "segment".into(),
        reference("the world segment it belongs to"),
    );
    m.insert(
        "cycle".into(),
        reference("the hero cycle it belongs to (instead of segment)"),
    );
    m.insert("step".into(), step());
    m.insert("text".into(), text("Longer description for organizers."));
    m.insert(
        "brief".into(),
        text("What players read about it on their sheet (no names)."),
    );
    m
}

fn action() -> Value {
    let mut p = node_common();
    if let Value::Object(f) = flow() {
        p.extend(f);
    }
    object(&["title"], &Value::Object(p))
}

fn decision() -> Value {
    let mut p = node_common();
    let mut option = serde_json::Map::new();
    option.insert("label".into(), text("What the option is called."));
    if let Value::Object(f) = flow() {
        option.extend(f);
    }
    p.insert(
        "option".into(),
        list_of(
            &object(&["label"], &Value::Object(option)),
            "The options, [[decision.option]].",
        ),
    );
    object(&["title"], &Value::Object(p))
}

fn phase() -> Value {
    object(
        &[],
        &json!({
            "segment": reference("the world segment the phase starts in"),
            "step": step(),
            "goals": texts("New goals from this moment (replace the old ones)."),
            "secrets": texts("New secrets from this moment."),
            "sheet": { "type": "object", "additionalProperties": { "type": "string" },
                       "description": "Sheet sections that change (appearance, whereabouts, backstory, …)." },
        }),
    )
}

fn person() -> Value {
    object(
        &["name", "role"],
        &json!({
            "name": text("Display name."),
            "role": { "enum": ["player", "organizer", "npc"],
                      "description": "player: gets a character sheet; organizer and npc: an organizer sheet." },
            "color": color(),
            "goals": texts("What the character wants."),
            "secrets": texts("What the character hides."),
            "sheet": { "type": "object", "additionalProperties": { "type": "string" },
                       "description": "Sheet sections: appearance, whereabouts, backstory, echo, or your own." },
            "relation": list_of(&object(&["with", "text"], &json!({
                "with": reference("the other person"),
                "text": text("What the character thinks of them."),
            })), "[[person.relation]]"),
            "played_by": reference("the player (players.toml) who plays this character"),
            "enters": reference("the world segment where the character enters the story"),
            "after": reference("the character whose death starts this one's life (reincarnation)"),
            "dies": { "description": "World segment id, or step number of the character's own cycle.",
                      "oneOf": [reference("a world segment"), step()] },
            "outside_cycle": { "type": "boolean", "description": "Never the hero of a cycle." },
            "phase": list_of(&phase(), "[[person.phase]]: how the sheet changes over time."),
        }),
    )
}

fn thing(kind: &str) -> Value {
    object(
        &["id", "name"],
        &json!({
            "id": id(kind),
            "name": text("Display name."),
            "category": reference("a category from items.toml"),
            "owners": refs("people or rooms that have it at the start"),
            "starting": { "type": "boolean", "description": "Exists when the game starts." },
            "ending": { "type": "boolean", "description": "Reaching it ends the game (an ending)." },
            "text": text("Description."),
        }),
    )
}

fn map() -> Value {
    object(
        &[],
        &json!({
            "cells": texts("Grid rows, top to bottom: `.` empty, `=` corridor."),
            "walls": list_of(&json!({ "type": "array", "items": { "type": "integer", "minimum": 0 },
                                      "minItems": 4, "maxItems": 4 }),
                             "Walls [x1, y1, x2, y2] along grid lines, horizontal or vertical."),
            "rooms": { "type": "object", "description": "Room id → place in cells.",
                       "additionalProperties": object(&["x", "y"], &json!({
                           "x": { "type": "integer", "minimum": 0 }, "y": { "type": "integer", "minimum": 0 },
                           "w": { "type": "integer", "minimum": 1 }, "h": { "type": "integer", "minimum": 1 },
                       })) },
        }),
    )
}

/// All schemas, one per kind of game file (world and time first, then content).
pub(crate) fn schemas() -> Vec<Schema> {
    vec![
        Schema {
            name: "game",
            globs: &["game.toml"],
            json: file(
                "game.toml",
                &["game"],
                &json!({
                    "game": object(&["title", "language"], &json!({
                        "title": text("Title of the game."),
                        "language": { "enum": ["cs", "en"], "description": "Language of the sheets." },
                        "steps": texts("Steps of a hero cycle, e.g. the 8 steps of the story circle."),
                    })),
                }),
            ),
        },
        Schema {
            name: "segments",
            globs: &["segments.toml"],
            json: file(
                "segments.toml",
                &[],
                &json!({
                    "segment": list_of(&object(&["id", "name"], &json!({
                        "id": id("the segment"), "name": text("Display name."), "color": color(),
                        "text": text("Notes for organizers."), "group": text("Group shown together in the viewer."),
                    })), "[[segment]]: world time, in order."),
                }),
            ),
        },
        Schema {
            name: "players",
            globs: &["players.toml"],
            json: file(
                "players.toml",
                &[],
                &json!({
                    "player": list_of(&object(&["id", "name"], &json!({
                        "id": id("the player"), "name": text("Display name."),
                    })), "[[player]]: real participants."),
                }),
            ),
        },
        Schema {
            name: "cycles",
            globs: &["cycles.toml"],
            json: file(
                "cycles.toml",
                &[],
                &json!({
                    "cycle": list_of(&object(&["id", "name", "hero", "from"], &json!({
                        "id": id("the cycle"), "name": text("Display name."),
                        "hero": reference("the character who goes through it"),
                        "from": reference("the world segment where it starts"),
                        "to": reference("the world segment where it ends (default: from)"),
                        "color": color(), "text": text("Notes for organizers."),
                    })), "[[cycle]]: one hero's journey on the world timeline."),
                }),
            ),
        },
    ]
    .into_iter()
    .chain(content_schemas())
    .collect()
}

/// Schemas of the files with game content: rooms, things, people, actions, layout, map.
fn content_schemas() -> Vec<Schema> {
    let point = object(
        &["x", "y"],
        &json!({ "x": { "type": "number" }, "y": { "type": "number" } }),
    );
    vec![
        Schema {
            name: "rooms",
            globs: &["rooms.toml"],
            json: file(
                "rooms.toml",
                &[],
                &json!({
                    "room": list_of(&object(&["id", "name"], &json!({
                        "id": id("the room"), "name": text("Display name."), "color": color(),
                    })), "[[room]]"),
                }),
            ),
        },
        Schema {
            name: "items",
            globs: &["items.toml"],
            json: file(
                "items.toml",
                &[],
                &json!({
                    "category": list_of(&object(&["id"], &json!({ "id": id("the category"), "color": color() })),
                                        "[[category]]"),
                    "item": list_of(&thing("the item"), "[[item]]: physical things."),
                }),
            ),
        },
        Schema {
            name: "info",
            globs: &["info.toml"],
            json: file(
                "info.toml",
                &[],
                &json!({
                    "info": list_of(&thing("the info"), "[[info]]: knowledge and story facts."),
                }),
            ),
        },
        Schema {
            name: "person",
            globs: &["people/*.toml"],
            json: file(
                "people/<id>.toml",
                &["person"],
                &json!({
                    "person": person(),
                }),
            ),
        },
        Schema {
            name: "action",
            globs: &["actions/*.toml"],
            json: {
                let mut v = file(
                    "actions/<id>.toml",
                    &[],
                    &json!({ "action": action(), "decision": decision() }),
                );
                v["oneOf"] = json!([{ "required": ["action"] }, { "required": ["decision"] }]);
                v
            },
        },
        Schema {
            name: "layout",
            globs: &["layout.toml"],
            json: file(
                "layout.toml",
                &[],
                &json!({
                    "node": { "type": "object", "additionalProperties": point, "description": "Node id → position." },
                    "knees": { "type": "object", "additionalProperties": list_of(&point, "Bends of a line."),
                               "description": "Line id → bends." },
                }),
            ),
        },
        Schema {
            name: "map",
            globs: &["map.toml", "map-presets/*.toml"],
            json: file(
                "map.toml",
                &["map"],
                &json!({
                    "map": map(),
                }),
            ),
        },
    ]
}

/// The `.taplo.toml` that applies the schemas in `.schemas/` to the game files.
fn taplo_config(schemas: &[Schema]) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "{MARKER}: editor autocomplete for this game (Taplo / Even Better TOML).\n\
         # Run `larpidarium schema` again after updating larpidarium.\n"
    );
    for s in schemas {
        let include: Vec<String> = s.globs.iter().map(|g| format!("\"{g}\"")).collect();
        let _ = write!(
            out,
            "\n[[rule]]\ninclude = [{}]\n[rule.schema]\npath = \"./.schemas/{}.schema.json\"\n",
            include.join(", "),
            s.name
        );
    }
    out
}

/// Runs the `schema` command: writes `<dir>/.schemas/*.schema.json` and `<dir>/.taplo.toml`.
pub(crate) fn run(dir: &Path) -> Result<()> {
    if !dir.join("game.toml").is_file() {
        return Err(miette!("{} has no game.toml", dir.display()));
    }
    let schemas = schemas();
    write_schemas(&dir.join(".schemas"), &schemas)?;
    let config = dir.join(".taplo.toml");
    match fs::read_to_string(&config) {
        Ok(old) if !old.starts_with(MARKER) => {
            println!(
                "✔ wrote {} schemas in {}; {} exists and is not ours, add these rules to it:\n\n{}",
                schemas.len(),
                dir.join(".schemas").display(),
                config.display(),
                taplo_config(&schemas)
            );
            return Ok(());
        }
        _ => fs::write(&config, taplo_config(&schemas))
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot write {}", config.display()))?,
    }
    println!(
        "✔ wrote {} schemas in {} and {}; open the game folder in an editor with Taplo \
         (VS Code: Even Better TOML) for autocomplete",
        schemas.len(),
        dir.join(".schemas").display(),
        config.display()
    );
    Ok(())
}

/// Writes every schema as `<folder>/<name>.schema.json`.
pub(crate) fn write_schemas(folder: &Path, schemas: &[Schema]) -> Result<()> {
    fs::create_dir_all(folder)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot create {}", folder.display()))?;
    for s in schemas {
        let path = folder.join(format!("{}.schema.json", s.name));
        fs::write(&path, render(s))
            .into_diagnostic()
            .wrap_err_with(|| format!("cannot write {}", path.display()))?;
    }
    Ok(())
}

/// Pretty JSON with a final newline.
pub(crate) fn render(s: &Schema) -> String {
    let mut text = serde_json::to_string_pretty(&s.json).unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The schema object that describes `key` inside `schema` (following arrays), if any.
    fn child<'a>(schema: &'a Value, key: &str) -> Option<&'a Value> {
        let schema = if schema["type"] == "array" {
            &schema["items"]
        } else {
            schema
        };
        schema["properties"]
            .get(key)
            .or_else(|| schema.get("additionalProperties").filter(|v| v.is_object()))
    }

    /// Every key path in `value` is described by `schema`.
    fn assert_described(value: &toml::Value, schema: &Value, path: &str) {
        match value {
            toml::Value::Table(t) => {
                // A free-form string map (`sheet`) accepts any key.
                if schema["additionalProperties"]["type"] == "string" {
                    return;
                }
                for (k, v) in t {
                    let sub = child(schema, k)
                        .unwrap_or_else(|| panic!("`{path}.{k}` is not in the schema"));
                    assert_described(v, sub, &format!("{path}.{k}"));
                }
            }
            toml::Value::Array(items) => {
                let item = if schema["type"] == "array" {
                    &schema["items"]
                } else {
                    schema
                };
                for v in items {
                    assert_described(v, item, path);
                }
            }
            _ => {}
        }
    }

    fn schema_for(path: &str) -> Option<Value> {
        let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
        schemas().into_iter().find_map(|s| {
            s.globs
                .iter()
                .any(|g| match g.split_once("/*") {
                    Some((folder, _)) => path.starts_with(&format!("{folder}/")),
                    None => *g == name && !path.contains('/'),
                })
                .then_some(s.json)
        })
    }

    #[test]
    fn every_key_in_the_examples_is_in_a_schema() {
        let mut checked = 0;
        for game in ["lost-abbey", "kolobeh", "evzentura"] {
            let dir = repo().join("examples").join(game);
            let files = crate::check::read_game_dir(&dir).unwrap();
            for f in files {
                let Some(schema) = schema_for(&f.path) else {
                    panic!("no schema for {game}/{}", f.path);
                };
                let value: toml::Value = toml::from_str(&f.text).unwrap();
                assert_described(&value, &schema, &format!("{game}/{}", f.path));
                checked += 1;
            }
        }
        assert!(checked > 50, "only {checked} files checked");
    }

    #[test]
    fn schemas_in_the_repo_are_up_to_date() {
        let folder = repo().join("schemas");
        if std::env::var_os("UPDATE_SCHEMAS").is_some() {
            write_schemas(&folder, &schemas()).unwrap();
        }
        for s in schemas() {
            let path = folder.join(format!("{}.schema.json", s.name));
            let committed = fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(
                committed,
                render(&s),
                "{} is stale; run `UPDATE_SCHEMAS=1 cargo test -p larpidarium schema`",
                path.display()
            );
        }
    }

    #[test]
    fn taplo_config_maps_every_schema() {
        let config = taplo_config(&schemas());
        assert!(config.starts_with(MARKER));
        let parsed: toml::Value = toml::from_str(&config).unwrap();
        let rules = parsed["rule"].as_array().unwrap();
        assert_eq!(rules.len(), schemas().len());
        assert!(config.contains("include = [\"people/*.toml\"]"));
        assert!(config.contains("path = \"./.schemas/person.schema.json\""));
    }

    #[test]
    fn run_writes_schemas_and_keeps_a_foreign_taplo_config() {
        let dir = std::env::temp_dir().join(format!("larpidarium-schema-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("game.toml"),
            "[game]\ntitle = \"x\"\nlanguage = \"cs\"\n",
        )
        .unwrap();
        run(&dir).unwrap();
        assert!(dir.join(".schemas/person.schema.json").is_file());
        let files = crate::check::read_game_dir(&dir).unwrap();
        assert!(
            files.iter().all(|f| !f.path.starts_with('.')),
            "hidden files are not game files"
        );
        assert!(
            fs::read_to_string(dir.join(".taplo.toml"))
                .unwrap()
                .starts_with(MARKER)
        );
        fs::write(dir.join(".taplo.toml"), "# mine\n").unwrap();
        run(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(".taplo.toml")).unwrap(),
            "# mine\n"
        );
        assert!(run(&dir.join("nope")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
