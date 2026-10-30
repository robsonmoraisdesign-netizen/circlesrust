//! Guard against replacing native product modules with script/C++ source files.
use std::{fs,path::Path};
fn inspect(path:&Path){
    for entry in fs::read_dir(path).unwrap(){
        let entry=entry.unwrap();let path=entry.path();
        if entry.file_type().unwrap().is_dir(){inspect(&path);continue;}
        let extension=path.extension().and_then(|v|v.to_str()).unwrap_or("");
        assert!(!["js","jsx","ts","tsx","mjs","cjs","html","py","cpp","cc","c"].contains(&extension),
            "Non-native implementation in product source: {}",path.display());
    }
}
#[test]
fn product_source_does_not_embed_replacement_script_modules(){
    inspect(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
}
