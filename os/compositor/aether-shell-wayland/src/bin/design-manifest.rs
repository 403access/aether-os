//! Print the compositor's design state as JSON.
//!
//! The parity tooling diffs this document against the Water OS demo's computed
//! styles, so chrome drift shows up as property values rather than pixels.

fn main() {
    let manifest = aether_shell_wayland::design_manifest::manifest();
    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}
