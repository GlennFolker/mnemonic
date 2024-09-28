use bevy::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[bevy_main]
fn main() {
    mnemonic::run();
}
