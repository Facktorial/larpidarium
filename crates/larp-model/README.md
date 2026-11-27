# larp-model

The typed model of a LARP game, and the only place that knows the TOML format.

- **Input:** the contents of the game folder (`game.toml`, `actions/*.toml`, `people/*.toml`, …).
- **Output:** a checked `GameModel`, or a list of errors with file and line.
- **No I/O:** functions take strings and return values. Reading files is the binary's job.

The format itself is documented for organizers in [`docs/format.md`](../../docs/format.md).

```rust
use larp_model::Id;

let id = Id::parse("abbey-gate")?;
assert_eq!(id.as_str(), "abbey-gate");
assert!(Id::parse("Abbey Gate").is_err());
# Ok::<(), larp_model::IdError>(())
```
