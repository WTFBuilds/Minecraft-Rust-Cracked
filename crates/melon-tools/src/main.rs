//! MELON tooling binary.
//!
//! Phase 7 gives this real subcommands (`arena import`, `arena inspect`);
//! Phase 11 adds the load-test bot driver. Kept as a stub so the workspace
//! shape is complete from day 1.

fn main() {
    println!(
        "melon-tools {} — MELON platform tooling\n\n\
         Planned subcommands:\n  \
         arena import   convert a vanilla-built map (.schematic/.mca) to a MELON arena (Phase 7)\n  \
         arena inspect  dump an arena's blocks, spawns and team data (Phase 7)\n  \
         bots run       drive headless clients as load-test bots (Phase 11)\n",
        env!("CARGO_PKG_VERSION")
    );
}
