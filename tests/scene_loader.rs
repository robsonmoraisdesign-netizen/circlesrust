use circlesrust::scene::loader::{SceneLoadOptions,load_scene};
use std::path::PathBuf;
#[test]
fn scene_loader_uses_one_pipeline_for_desktop_and_cli(){
    let default=load_scene(&SceneLoadOptions::studio()).unwrap();
    assert_eq!(default.material_names.len(),default.scene.materials.len());
    assert!(default.scene.bvh.stats().triangles>10);
    let loaded=load_scene(&SceneLoadOptions{
        obj:Some(PathBuf::from("tests/fixtures/textured.obj")),
        mtl:Some(PathBuf::from("tests/fixtures/textured.mtl")),
        ..SceneLoadOptions::studio()
    }).unwrap();
    assert_eq!(loaded.scene.materials.len(),3);
    assert_eq!(loaded.scene.shader_graphs.len(),3);
    assert!(loaded.scene.shader_graphs[2].is_some());
}
#[test]
fn invalid_paths_and_strength_do_not_start_renderer(){
    assert!(load_scene(&SceneLoadOptions{
        obj:Some(PathBuf::from("not_found.obj")),..SceneLoadOptions::studio()
    }).is_err());
}
