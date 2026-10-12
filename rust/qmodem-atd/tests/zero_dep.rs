//! Zero-dependency guard: the whole workspace must stay free of external
//! crates (the "single-package delivery" strategy). CI can also enforce
//! this at the manifest level; this test keeps it honest locally.

use std::path::Path;

fn read_manifest(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel).join("Cargo.toml");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}

#[test]
fn manifests_declare_no_external_crates() {
    let atd = read_manifest(".");
    let pdu = read_manifest("../qmodem-pdu");
    let workspace = read_manifest("..");
    for (name, text) in [
        ("qmodem-atd", &atd),
        ("qmodem-pdu", &pdu),
        ("workspace", &workspace),
    ] {
        for banned in ["tokio", "serde", "thiserror", "libc"] {
            assert!(
                !text.contains(banned),
                "{name} Cargo.toml must not reference `{banned}`"
            );
        }
        assert!(
            !text.contains("[dependencies]"),
            "{name} Cargo.toml must not declare a [dependencies] section"
        );
    }
}
