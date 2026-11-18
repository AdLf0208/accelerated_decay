# Working on this Horizon native mod

This is a **flat SDK package** (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs`
(`impl mod_api::Hooks`) + `src/runtime.rs` (the pure rules and their tests). Horizon retired the
`.hnmod` container in the mod-API merge (#16941), so there is no `compatibility.json`,
`mixins/<id>/`, `horizon-mixin.toml`, plan/injection schema or `rust-toolchain.toml` here, and none
of them should be recreated. The retired 0.1.0 tree is kept as evidence at
`../_shared/evidence/accelerateddecay/retired-hnmod-0.1.0/`.

> **Build slots (v2):** every build/test goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a slot is
> free, so **never poll and never background a build**; a timeout exits **75**. It does **not**
> `cd` into the port — wrap the command (`bash -c 'cd ports/<id> && …'`) and never pass
> `--target-dir`.

```sh
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step test \
    -- bash -c 'cd ports/accelerateddecay && CARGO_INCREMENTAL=0 cargo test'
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step check \
    -- bash -c 'cd ports/accelerateddecay && cargo horizon-mod check accelerateddecay.hmod'
```

- **`src/runtime.rs` is byte-identical to the retired bundle's `runtime.rs` and must stay so**
  (SHA-256 `6f78cfec40a2ab6bee8a76067bf6582208c4b771ae7af3b6db08050538729644`): the rules live
  there with their 15 tests, and `src/lib.rs` only converts SDK types to theirs. New rule → a test
  in `runtime.rs`; new glue → a private function and a test in `lib.rs`.
- Only the API the SDK publishes: `cargo horizon-mod hooks <text>` finds the exact method; never
  guess a name, and record an API gap in `notes.md` instead of faking one.
- The delivered package is `accelerateddecay.hmod` **in the port root**, never in `target/`;
  `LICENSE` and `LICENSE-NOTICE.md` travel inside it.
- Keep the port separate from Horizon and horizon-mixins; never modify the pin checkouts, never
  commit binaries, exported sources or Minecraft assets, and never put Mojang files in the tree.
- `python3 ports/tools/check-skills.py --notes ports/accelerateddecay` must stay green, and
  `notes.md` records every claim with its command and output.
