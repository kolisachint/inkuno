// Nothing to embed: the Linux desktop file and icons install from the sources
// (see the Makefile), and the macOS bundle takes its icon and property list
// from ../../packaging/macos (see scripts/bundle-macos.sh).
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
