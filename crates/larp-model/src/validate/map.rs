//! Checks of `map.toml` and `map-presets/*.toml`: valid tiles and walls, known rooms, no
//! overlapping rooms.

use crate::Diagnostic;
use crate::model::{Game, Map};

/// Tile characters allowed in `map.cells`.
const TILES: [char; 2] = ['.', '='];

pub(super) fn check(game: &Game, diags: &mut Vec<Diagnostic>) {
    for (id, preset) in &game.map_presets {
        check_shape(&format!("map-presets/{id}.toml"), preset, diags);
    }
    let Some(map) = &game.map else {
        return;
    };
    let file = "map.toml";
    check_shape(file, map, diags);
    for (id, place) in &map.rooms {
        if !game.rooms.iter().any(|r| r.id.get_ref() == id) {
            diags.push(
                Diagnostic::error(
                    file,
                    Some(place.span()),
                    format!("unknown room `{id}` on the map"),
                )
                .with_help("rooms are defined in rooms.toml"),
            );
        }
    }
    let placed: Vec<_> = map.rooms.iter().collect();
    for (i, (a_id, a)) in placed.iter().enumerate() {
        let span = a.span();
        let a = a.get_ref();
        if a.w == 0 || a.h == 0 {
            diags.push(Diagnostic::error(
                file,
                Some(span.clone()),
                format!("room `{a_id}` has zero size on the map"),
            ));
        }
        for (b_id, b) in placed.iter().skip(i + 1) {
            let b = b.get_ref();
            // Widened: x + w can exceed u16.
            let end = |start: u16, len: u16| u32::from(start) + u32::from(len);
            let apart = end(a.x, a.w) <= u32::from(b.x)
                || end(b.x, b.w) <= u32::from(a.x)
                || end(a.y, a.h) <= u32::from(b.y)
                || end(b.y, b.h) <= u32::from(a.y);
            if !apart {
                diags.push(Diagnostic::warning(
                    file,
                    Some(span.clone()),
                    format!("rooms `{a_id}` and `{b_id}` overlap on the map"),
                ));
            }
        }
    }
}

/// Tiles and walls, shared by the map and its presets.
fn check_shape(file: &str, map: &Map, diags: &mut Vec<Diagnostic>) {
    for (row, line) in map.cells.iter().enumerate() {
        if let Some(bad) = line.get_ref().chars().find(|c| !TILES.contains(c)) {
            diags.push(
                Diagnostic::error(
                    file,
                    Some(line.span()),
                    format!("row {}: unknown tile `{bad}`", row + 1),
                )
                .with_help("use `.` (empty) or `=` (corridor); walls go in `walls`"),
            );
        }
    }
    for wall in &map.walls {
        let [x1, y1, x2, y2] = *wall.get_ref();
        if (x1 != x2 && y1 != y2) || (x1 == x2 && y1 == y2) {
            diags.push(
                Diagnostic::error(
                    file,
                    Some(wall.span()),
                    "a wall must be a horizontal or vertical line",
                )
                .with_help("`[x1, y1, x2, y2]` with x1 = x2 or y1 = y2, between two grid points"),
            );
        }
    }
}
