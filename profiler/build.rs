use std::{
    env::var_os,
    fs::{read_dir, write},
    path::PathBuf,
};

use libbpf_cargo::SkeletonBuilder;

fn main() {
    let out_dir = var_os("OUT_DIR").expect("OUT_DIR must be set in build script");
    let out = PathBuf::from(out_dir);

    let mut mod_rs = String::new();

    for entry in read_dir("src/bpf").unwrap() {
        let path = entry.unwrap().path();

        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        if !name.ends_with(".bpf.c") {
            continue;
        }

        println!("cargo:rerun-if-changed={}", path.display());

        let module = name.trim_end_matches(".bpf.c");
        let skel_path = out.join(format!("{module}.skel.rs"));

        SkeletonBuilder::new()
            .source(&path)
            .build_and_generate(&skel_path)
            .unwrap();

        mod_rs.push_str(&format!(
            "pub mod {} {{ include!(concat!(env!(\"OUT_DIR\"), \"/{}.skel.rs\")); }}\n",
            module, module
        ));
    }

    write(out.join("mod.rs"), mod_rs).unwrap();
}

