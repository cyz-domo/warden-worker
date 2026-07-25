use std::{fs, path::Path};

fn main() {
    let version_path = Path::new("static/web-vault/version.json");
    println!("cargo:rerun-if-changed={}", version_path.display());

    let contents = fs::read_to_string(version_path).expect("failed to read Web Vault version");
    let marker = "\"version\"";
    let value = contents
        .split_once(marker)
        .and_then(|(_, rest)| {
            let colon = rest.find(':')?;
            let quoted = &rest[colon + 1..];
            let start = quoted.find('"')? + 1;
            let end = quoted[start..].find('"')? + start;
            Some(&quoted[start..end])
        })
        .filter(|version| !version.is_empty())
        .expect("static/web-vault/version.json must contain a version string");

    println!("cargo:rustc-env=WARDEN_VERSION={value}");
}
