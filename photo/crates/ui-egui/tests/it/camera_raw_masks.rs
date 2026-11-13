//! Camera Raw's Local Adjustments through the real dialog: the panel adds, selects and deletes
//! masks, the preview's handles move them, and OK develops the picture through them.
use egui::accesskit::Role;
use egui::{Event, PointerButton, Pos2, Rect, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use photocraft_ui_egui::{PhotocraftApp, camera_raw_ui, control, theme::ThemeKind};
use serde_json::{Value, json};

fn fixture() -> Harness<'static, PhotocraftApp> {
    let mut h = Harness::builder().with_step_dt(1.0 / 60.0).with_size(vec2(1300.0, 900.0)).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, ThemeKind::Pro);
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width":400,"height":300})).unwrap();
        app.run("layer.new.layer", json!({})).unwrap();
        app.run("edit.fill", json!({"color":"#808080"})).unwrap();
        camera_raw_ui::open(&mut app, &cc.egui_ctx).unwrap();
        app
    });
    h.ctx.all_styles_mut(|style| style.animation_time = 0.0);
    h.run_steps(4);
    h
}

fn inspect(h: &Harness<'_, PhotocraftApp>) -> Value {
    control::inspect(h.state(), &h.ctx)["cameraRaw"].clone()
}

fn masks(h: &Harness<'_, PhotocraftApp>) -> Vec<Value> {
    inspect(h)["params"]["masks"].as_array().cloned().unwrap_or_default()
}

fn ui(h: &mut Harness<'_, PhotocraftApp>, request: Value) -> Result<Value, String> {
    let ctx = h.ctx.clone();
    let r = camera_raw_ui::menu(h.state_mut(), &ctx, "filter.cameraRaw", &json!({"ui": request})).unwrap();
    h.run_steps(4);
    r
}

fn preview_rect(h: &Harness<'_, PhotocraftApp>) -> Rect {
    let v = inspect(h);
    let a = v["previewRect"].as_array().expect("the preview is on screen");
    let n = |i: usize| a[i].as_f64().unwrap() as f32;
    Rect::from_min_max(pos2(n(0), n(1)), pos2(n(2), n(3)))
}

fn at(rect: Rect, f: [f32; 2]) -> Pos2 {
    pos2(rect.left() + f[0] * rect.width(), rect.top() + f[1] * rect.height())
}

fn drag(h: &mut Harness<'_, PhotocraftApp>, start: Pos2, end: Pos2) {
    let m = egui::Modifiers::NONE;
    h.event(Event::PointerMoved(start));
    h.run_steps(2);
    h.event(Event::PointerButton { pos: start, button: PointerButton::Primary, pressed: true, modifiers: m });
    h.run_steps(2);
    for n in 1..=5 {
        h.event(Event::PointerMoved(start + (end - start) * n as f32 / 5.0));
        h.run_steps(2);
    }
    h.event(Event::PointerButton { pos: end, button: PointerButton::Primary, pressed: false, modifiers: m });
    h.run_steps(3);
}

#[test]
fn the_panel_adds_selects_and_deletes_masks() {
    let mut h = fixture();
    assert!(masks(&h).is_empty());
    h.get_by_role_and_label(Role::DisclosureTriangle, "Local Adjustments").click();
    h.run_steps(3);
    h.get_by_label("Linear Gradient").click();
    h.run_steps(3);
    assert_eq!(masks(&h).len(), 1);
    assert_eq!(masks(&h)[0]["shape"], "linear");
    assert_eq!(inspect(&h)["maskSelected"], 0);
    h.get_by_label("Radial Gradient").click();
    h.run_steps(3);
    assert_eq!(masks(&h).iter().map(|m| m["shape"].as_str().unwrap().to_string()).collect::<Vec<_>>(), ["linear", "radial"]);
    assert_eq!(inspect(&h)["maskSelected"], 1, "a new mask is the one being edited");
    // the selected mask's sliders are listed
    for label in ["Amount", "Center X", "Feather", "Exposure", "Dehaze"] {
        // some labels also name a global slider, so one or more matches count
        assert!(h.query_all_by_role_and_label(Role::Slider, label).next().is_some(), "{label}");
    }
    h.get_by_label("Linear Gradient 1").click();
    h.run_steps(3);
    assert_eq!(inspect(&h)["maskSelected"], 0);
    assert!(h.query_all_by_role_and_label(Role::Slider, "Start X").next().is_some());
    // deleting the first leaves the radial one, now selected
    h.get_all_by_label("Delete").next().unwrap().click();
    h.run_steps(3);
    let left = masks(&h);
    assert_eq!((left.len(), left[0]["shape"].as_str()), (1, Some("radial")));
    assert_eq!(inspect(&h)["maskSelected"], 0);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert!(masks(&h).is_empty());
    assert_eq!(inspect(&h)["maskSelected"], Value::Null);
}

#[test]
fn dragging_a_handle_on_the_preview_moves_the_mask() {
    let mut h = fixture();
    ui(&mut h, json!({"set": {"masks": [{"shape": "linear", "start": [0.5, 0.25], "end": [0.5, 0.75], "exposure": 1.0}]}})).unwrap();
    assert_eq!(inspect(&h)["maskSelected"], 0, "the first mask is the one shown");
    let image = preview_rect(&h);
    drag(&mut h, at(image, [0.5, 0.25]), at(image, [0.5, 0.1]));
    let m = &masks(&h)[0];
    let start = m["start"].as_array().unwrap();
    assert!((start[1].as_f64().unwrap() - 0.1).abs() < 0.03, "start moved up: {start:?}");
    assert!((start[0].as_f64().unwrap() - 0.5).abs() < 0.03, "and stayed in its column: {start:?}");
    assert_eq!(m["end"], json!([0.5, 0.75]), "the other end did not move");
    // the radial handles: the centre moves, a half-axis handle resizes
    ui(&mut h, json!({"set": {"masks": [{"shape": "radial", "center": [0.5, 0.5], "radius": [0.2, 0.2], "feather": 0.0, "exposure": 1.0}]}})).unwrap();
    let image = preview_rect(&h);
    drag(&mut h, at(image, [0.5, 0.5]), at(image, [0.3, 0.4]));
    let c = masks(&h)[0]["center"].clone();
    assert!((c[0].as_f64().unwrap() - 0.3).abs() < 0.03 && (c[1].as_f64().unwrap() - 0.4).abs() < 0.03, "{c}");
    let image = preview_rect(&h);
    // the width handle is at centre + radius.x to the right
    drag(&mut h, at(image, [0.3 + 0.2, 0.4]), at(image, [0.3 + 0.35, 0.4]));
    let r = masks(&h)[0]["radius"][0].as_f64().unwrap();
    assert!((r - 0.35).abs() < 0.03, "width handle: {r}");
}

#[test]
fn ok_develops_the_picture_through_the_masks() {
    let mut h = fixture();
    ui(&mut h, json!({"set": {"masks": [{"shape": "linear", "start": [0.5, 0.2], "end": [0.5, 0.6], "exposure": 1.5}]}})).unwrap();
    ui(&mut h, json!({"commit": true})).unwrap();
    let st = h.state().session.active().unwrap();
    let surface = st.active_layer.and_then(|id| st.doc.layer(id)).unwrap().surface().unwrap();
    let (top, middle, bottom) = (surface.rgba(200, 10)[0], surface.rgba(200, 120)[0], surface.rgba(200, 290)[0]);
    let grey = surface.rgba(200, 290)[0];
    assert!(top > grey + 0.1, "the top is brighter: {top} vs {grey}");
    assert!(middle > bottom && middle < top, "the ramp is between: {top} {middle} {bottom}");
    assert!((bottom - 128.0 / 255.0).abs() < 0.01, "the bottom is the original grey: {bottom}");
}

#[test]
fn malformed_masks_are_refused_by_the_dialog_and_old_settings_still_open() {
    let mut h = fixture();
    for bad in [json!([{"shape": "linear", "exposure": 9.0}]), json!([{"shape": "radial", "radius": [0.0, 0.2]}]), json!(vec![json!({"shape": "linear"}); 17])]
    {
        let e = ui(&mut h, json!({"set": {"masks": bad.clone()}}));
        assert!(e.is_err(), "{bad}");
        assert!(masks(&h).is_empty());
    }
    assert!(ui(&mut h, json!({"set": {"exposure": 0.5}})).is_ok(), "settings without masks still apply");
}
