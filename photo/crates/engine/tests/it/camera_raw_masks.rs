//! `filter.cameraRaw` with local adjustments (`masks`): the command develops them, bounds them,
//! refuses malformed ones, and keeps them as a smart filter's settings.

use photocraft_engine::Session;
use serde_json::json;

fn session_with_grey_layer(depth: u32) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width": 40, "height": 40, "depth": depth})).unwrap();
    s.execute("layer.new.layer", json!({})).unwrap();
    s.execute("edit.fill", json!({"contents": "color", "color": "#808080"})).unwrap();
    s
}

fn red(s: &Session, x: u32, y: u32) -> f32 {
    let st = s.active().unwrap();
    let layer = st.active_layer.and_then(|id| st.doc.layer(id)).unwrap();
    layer.surface().unwrap().rgba(x as i32, y as i32)[0]
}

fn top_brightened() -> serde_json::Value {
    json!({"masks": [{"shape": "linear", "start": [0.5, 0.1], "end": [0.5, 0.5], "exposure": 1.0}]})
}

#[test]
fn a_mask_develops_only_where_it_covers() {
    let mut s = session_with_grey_layer(8);
    let before = red(&s, 20, 2);
    let out = s.execute("filter.cameraRaw", top_brightened()).unwrap();
    assert_eq!(out["identity"], false, "a mask with sliders is not an identity");
    assert!(red(&s, 20, 2) > before + 0.05, "the top is brighter");
    assert_eq!(red(&s, 20, 38), before, "the bottom is untouched");
}

#[test]
fn masks_that_do_nothing_are_an_identity() {
    let mut s = session_with_grey_layer(8);
    let out = s.execute("filter.cameraRaw", json!({"masks": [{"shape": "radial"}, {"shape": "linear", "enabled": false, "exposure": 2.0}]})).unwrap();
    assert_eq!(out["identity"], true);
}

#[test]
fn malformed_masks_are_refused_and_stored_ones_are_capped() {
    let mut s = session_with_grey_layer(8);
    for bad in [
        json!({"masks": [{"shape": "linear", "exposure": 50.0}]}),
        json!({"masks": [{"shape": "radial", "feather": 500.0}]}),
        json!({"masks": [{"shape": "radial", "radius": [0.0, 0.3]}]}),
        json!({"masks": [{"shape": "triangle"}]}),
        json!({"masks": vec![json!({"shape": "linear", "exposure": 0.1}); 17]}),
    ] {
        let e = s.execute("filter.cameraRaw", bad.clone());
        assert!(e.is_err(), "{bad} was accepted");
    }
    // a stored (lenient) list longer than the cap is cut, not rejected
    let many = json!({"masks": vec![json!({"shape": "radial", "exposure": 0.1}); 40]});
    let cr = photocraft_engine::lens_cmds::raw_params("filter.cameraRaw", &many).unwrap();
    assert_eq!(cr.masks.len(), photocraft_algo::camera_raw::masks::MAX_MASKS);
}

#[test]
fn masks_keep_a_32_bit_layer_finite() {
    let mut s = session_with_grey_layer(32);
    s.execute("filter.cameraRaw", json!({"masks": [{"shape": "radial", "exposure": 5.0, "highlights": 100.0, "dehaze": 100.0, "clarity": 100.0}]})).unwrap();
    for y in [0, 20, 39] {
        for x in [0, 20, 39] {
            assert!(red(&s, x, y).is_finite(), "{x},{y}");
        }
    }
}
