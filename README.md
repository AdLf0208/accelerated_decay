# Accelerated Decay — a Horizon native mod (SDK format)

A port of **Accelerated Decay 21.0.0** (nanite/AcceleratedDecay, original author ErrorMikey) to
Horizon 26.3, written as one flat SDK package (mod API `0.2`).

One real second after a player fells a `#minecraft:logs` block, the leaves around it that are
already ready to decay (`distance == 7`, not `persistent`) are removed in a single flood fill over
the 26-neighbour cube, and a waterlogged leaf leaves `minecraft:water` behind. The game's own
distance machinery still decides which leaves are ready; this mod only stops waiting for the random
tick.

**Licence: GPL-3.0-only** — the jar's `META-INF/neoforge.mods.toml` declares `license = "GPL3"`, so
the artefact is the authority over the repository's stale MIT text (see `LICENSE-NOTICE.md`).
Local-only until the licence terms and the maintainer's approval allow publishing
(`ARCHITECTURE.md` §8 of the campaign).

## Layout

```text
accelerateddecay.hmod          # the delivered package (port root, never target/)
  horizon-mod.toml             # the only manifest: id/version/api/side/capabilities + [native] pins
  Cargo.toml                   # depends on horizon-mod-api from .horizon/sdk/ (fetched, never shipped)
  src/lib.rs                   # the one #[mod_api::horizon_mod] impl Hooks (the glue)
  src/runtime.rs               # the pure engine and its 15 tests — byte-identical to the 0.1.0 file
  LICENSE, LICENSE-NOTICE.md   # the licence travels inside the package
  uses.json                    # generated: the SDK surface the sources name
tools/e2e.sh                   # the in-game scenario (needs a launcher-built game)
notes.md                       # behaviour, class/tier, gaps, verification, the migration recipe
```

## Build, test, check

```sh
# from the campaign root; the runner blocks until a slot is free and does not cd for you
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step test \
    -- bash -c 'cd ports/accelerateddecay && CARGO_INCREMENTAL=0 cargo test'
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step lint \
    -- bash -c 'cd ports/accelerateddecay && cargo fmt --check && cargo clippy --all-targets -- -D warnings'
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step build \
    -- bash -c 'cd ports/accelerateddecay && rm -f Cargo.lock && cargo horizon-mod build &&
        cp target/horizon-mod/accelerateddecay.hmod .'
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step check \
    -- bash -c 'cd ports/accelerateddecay && cargo horizon-mod check accelerateddecay.hmod &&
        cargo horizon-mod fits accelerateddecay.hmod --build nightly'
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never packed). `build`
packs the `.hmod`; it does not compile against the game — that is
`horizon-launcher native-build . --instance <instance> --trust-native-code`, which is what
`tools/e2e.sh` needs (`GAME=<launcher-built horizon-client>`).

There is no `Cargo.lock` in the package on purpose: the crate has no registry dependency of its
own, and a lock would ship resolved versions the SDK was not built against.
