use circlesrust::scene::{history::TransformHistory, studio_scene, transform::ObjectTransform};
use circlesrust::util::math::Vec3;

fn translated(x: f32) -> ObjectTransform {
    ObjectTransform { translation: Vec3::new(x, 0.0, 0.0), ..Default::default() }
}

#[test]
fn whole_gesture_undo_redo_restores_exact_mesh_and_preserves_other_objects() {
    let mut scene = studio_scene();
    let mut history = TransformHistory::default();
    let before: Vec<_> = (0..scene.bvh.stats().triangles)
        .map(|i| scene.bvh.triangle(i).unwrap().a).collect();
    for x in [0.2, 0.4, 0.8] { history.apply(&mut scene, 4, translated(x)).unwrap(); }
    history.finish();
    assert_eq!(history.undo(&mut scene).unwrap(), Some(4));
    assert!(!history.can_undo());
    for (i, point) in before.iter().enumerate() { assert_eq!(scene.bvh.triangle(i).unwrap().a, *point); }
    assert_eq!(history.redo(&mut scene).unwrap(), Some(4));
    assert_eq!(scene.object_transforms[4], translated(0.8));
    for (i, object) in scene.triangle_objects.iter().enumerate() {
        if *object != 4 { assert_eq!(scene.bvh.triangle(i).unwrap().a, before[i]); }
    }
}

#[test]
fn cancellation_and_invalid_edits_keep_redo_but_committed_edit_branches() {
    let mut scene = studio_scene();
    let mut history = TransformHistory::default();
    history.apply(&mut scene, 4, translated(1.0)).unwrap();
    history.undo(&mut scene).unwrap();
    assert!(history.apply(&mut scene, 4, translated(f32::NAN)).is_err());
    assert!(!history.apply(&mut scene, 4, ObjectTransform::default()).unwrap());
    assert!(history.can_redo());
    history.apply(&mut scene, 4, translated(2.0)).unwrap();
    history.cancel(&mut scene).unwrap();
    assert_eq!(scene.object_transforms[4], ObjectTransform::default());
    assert!(history.can_redo());
    history.apply(&mut scene, 3, translated(3.0)).unwrap();
    history.finish();
    assert!(!history.can_redo());
    assert_eq!(history.undo(&mut scene).unwrap(), Some(3));
}

#[test]
fn object_switch_splits_commands_and_history_is_bounded() {
    let mut scene = studio_scene();
    let mut history = TransformHistory::default();
    history.apply(&mut scene, 3, translated(1.0)).unwrap();
    history.apply(&mut scene, 4, translated(2.0)).unwrap();
    assert_eq!(history.undo(&mut scene).unwrap(), Some(4));
    assert_eq!(history.undo(&mut scene).unwrap(), Some(3));
    for n in 1..=130 {
        history.apply(&mut scene, 3, translated(n as f32 / 10.0)).unwrap();
        history.finish();
    }
    let mut count = 0;
    while history.undo(&mut scene).unwrap().is_some() { count += 1; }
    assert_eq!(count, 128);
}

#[test]
fn undo_and_redo_restore_render_pixels_not_only_transform_values() {
    use circlesrust::integrator::{render_rgb,RenderSettings};
    let mut scene=studio_scene();let mut history=TransformHistory::default();
    let settings=RenderSettings{width:24,height:16,samples:2,max_bounces:3,exposure:1.0,seed:31};
    let original=render_rgb(&scene,settings).unwrap();
    let changed=ObjectTransform{translation:Vec3::new(1.0,0.0,0.2),
        rotation_degrees:Vec3::new(35.0,20.0,45.0),scale:Vec3::new(1.5,0.7,1.0)};
    history.apply(&mut scene,4,changed).unwrap();history.finish();
    let after=render_rgb(&scene,settings).unwrap();assert_ne!(original,after);
    history.undo(&mut scene).unwrap();assert_eq!(render_rgb(&scene,settings).unwrap(),original);
    history.redo(&mut scene).unwrap();assert_eq!(render_rgb(&scene,settings).unwrap(),after);
}
