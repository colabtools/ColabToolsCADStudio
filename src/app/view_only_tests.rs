//! View-only sessions (an embedding page opens a drawing read-only, or
//! `?viewonly=1`): every change to the drawing is taken back as soon as it is
//! recorded, while viewing (zoom, layouts, layer on/off and freeze) stays free.

use super::*;
use document::entities::{Circle, Line};
use document::types::Vector3;
use document::EntityType;

fn drawing(view_only: bool) -> OpenCADStudio {
    let mut app = OpenCADStudio::new_for_test();
    let _ = app.automation_op(r#"{"op":"new"}"#);
    let _ = app.automation_op(r#"{"op":"run","cmd":"-LAYER NEW Walls"}"#);
    let tab = app.active_tab;
    let scene = &mut app.tabs[tab].scene;
    let mut wall = EntityType::Line(Line::from_points(
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
    ));
    wall.common_mut().layer = "Walls".to_string();
    let _ = scene.add_entity(wall);
    let _ = scene.add_entity(EntityType::Circle(Circle::from_coords(20.0, 5.0, 0.0, 3.0)));
    app.tabs[tab].history = Default::default();
    app.view_only = view_only;
    app.read_only = view_only;
    app
}

fn entity_count(app: &OpenCADStudio) -> usize {
    app.tabs[app.active_tab].scene.document.entities().count()
}

fn walls(app: &OpenCADStudio) -> document::tables::Layer {
    app.tabs[app.active_tab]
        .scene
        .document
        .layers
        .get("Walls")
        .cloned()
        .expect("Walls layer")
}

fn run(app: &mut OpenCADStudio, cmd: &str) -> serde_json::Value {
    app.automation_op(&serde_json::json!({"op": "run", "cmd": cmd}).to_string())
}

#[test]
fn erase_is_taken_back_in_a_view_only_session() {
    let mut app = drawing(true);
    let before = entity_count(&app);
    assert_eq!(before, 2);
    let _ = run(&mut app, "ERASE ALL ENTER");
    assert_eq!(entity_count(&app), before, "nothing is erased");
    let tab = app.active_tab;
    assert!(app.tabs[tab].history.undo_stack.is_empty(), "no undo step is left");
    assert!(app.tabs[tab].history.redo_stack.is_empty(), "no redo step is left");
}

#[test]
fn drawing_new_objects_is_taken_back_in_a_view_only_session() {
    let mut app = drawing(true);
    let _ = run(&mut app, "CIRCLE 50,50 4");
    let _ = run(&mut app, "LINE 0,0 5,5 ENTER");
    assert_eq!(entity_count(&app), 2);
}

#[test]
fn layer_properties_other_than_visibility_are_taken_back() {
    let mut app = drawing(true);
    let colour = walls(&app).color.clone();
    let _ = run(&mut app, "-LAYER COLOR 1 Walls ");
    assert_eq!(walls(&app).color, colour);
    let _ = run(&mut app, "-LAYER NEW Doors");
    assert!(app.tabs[app.active_tab].scene.document.layers.get("Doors").is_none());
}

#[test]
fn layer_off_and_freeze_stay_in_a_view_only_session() {
    let mut app = drawing(true);
    let _ = run(&mut app, "-LAYER OFF Walls ");
    assert!(walls(&app).flags.off, "the layer is off");
    let _ = run(&mut app, "-LAYER ON Walls ");
    let _ = run(&mut app, "-LAYER FREEZE Walls ");
    assert!(walls(&app).flags.frozen, "the layer is frozen");
    assert_eq!(entity_count(&app), 2);
}

#[test]
fn zoom_and_layouts_stay_in_a_view_only_session() {
    let mut app = drawing(true);
    let zoom = run(&mut app, "ZOOM E");
    assert_eq!(zoom["status"], "completed", "{zoom}");
    let _ = run(&mut app, "TILEMODE 0");
    let tab = app.active_tab;
    assert_ne!(app.tabs[tab].scene.current_layout, "Model", "a paper layout is current");
    let _ = run(&mut app, "TILEMODE 1");
    assert_eq!(app.tabs[tab].scene.current_layout, "Model");
    // Opening an empty paper layout adds its own default viewports (never
    // saved here); what was drawn is untouched.
    let drawn = app.tabs[tab]
        .scene
        .document
        .entities()
        .filter(|e| matches!(e, EntityType::Line(_) | EntityType::Circle(_)))
        .count();
    assert_eq!(drawn, 2);
}

#[test]
fn a_normal_session_still_edits() {
    let mut app = drawing(false);
    let _ = run(&mut app, "ERASE ALL ENTER");
    assert_eq!(entity_count(&app), 0);
    let _ = run(&mut app, "-LAYER COLOR 1 Walls ");
    assert_eq!(app.tabs[app.active_tab].history.undo_stack.len(), 2);
}
