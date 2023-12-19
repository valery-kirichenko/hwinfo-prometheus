use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest(new_manifest("ValeryKirichenko.HWiNFOPrometheus"))
            .expect("unable to embed manifest file");

        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set_language(0x0409); // English (US)
        res.compile().unwrap();
    }
    println!("cargo:rerun-if-changed=build.rs");
}