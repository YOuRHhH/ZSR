use super::*;
use egui::{Event, Modifiers, PointerButton};

fn pointer(position: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}
fn frame(ctx: &egui::Context, picker: &mut Picker, events: Vec<Event>) {
    let bounds = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(bounds),
            events,
            ..Default::default()
        },
        |ctx| picker.interact(ctx, bounds),
    );
}
fn picker() -> Picker {
    Picker {
        selection: Some(Rect::from_min_max(
            Pos2::new(100.0, 100.0),
            Pos2::new(300.0, 200.0),
        )),
        canvas_size: Vec2::new(1000.0, 800.0),
        ..Default::default()
    }
}

#[test]
fn picker_initializes_display_pixels_and_escape_cancels() {
    let ctx = egui::Context::default();
    let mut picker = Picker::default();
    let display = Display {
        x: -1920,
        y: 0,
        width: 1920,
        height: 1080,
        id: 0,
    };
    let initial = Region {
        x: 200,
        y: 200,
        width: 800,
        height: 600,
    };
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0))),
            ..Default::default()
        },
        |ctx| {
            assert!(
                picker
                    .show(ctx, display, initial, Language::English)
                    .is_none()
            );
        },
    );
    let region = picker.region(display).unwrap();
    assert_eq!(
        (region.x, region.y, region.width, region.height),
        (200, 200, 800, 600)
    );
    let _ = ctx.run(
        egui::RawInput {
            events: vec![Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ctx| {
            assert!(matches!(
                picker.show(ctx, display, initial, Language::English),
                Some(Outcome::Cancel)
            ));
        },
    );
}

#[test]
fn drag_outside_draws_a_new_frame() {
    let ctx = egui::Context::default();
    let mut picker = picker();
    frame(&ctx, &mut picker, vec![]);
    let start = Pos2::new(600.0, 300.0);
    let end = Pos2::new(800.0, 500.0);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&ctx, &mut picker, vec![Event::PointerMoved(end)]);
    frame(&ctx, &mut picker, vec![pointer(end, false)]);
    assert_eq!(picker.selection, Some(Rect::from_min_max(start, end)));
    assert!(picker.drag.is_none());
}

#[test]
fn moving_frame_clamps_to_canvas_without_resizing() {
    let ctx = egui::Context::default();
    let mut picker = picker();
    frame(&ctx, &mut picker, vec![]);
    let start = Pos2::new(200.0, 150.0);
    let end = Pos2::new(990.0, 790.0);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&ctx, &mut picker, vec![Event::PointerMoved(end)]);
    frame(&ctx, &mut picker, vec![pointer(end, false)]);
    assert_eq!(
        picker.selection.unwrap(),
        Rect::from_min_size(Pos2::new(800.0, 700.0), Vec2::new(200.0, 100.0))
    );
}

#[test]
fn corner_drag_resizes_from_opposite_anchor() {
    let ctx = egui::Context::default();
    let mut picker = picker();
    frame(&ctx, &mut picker, vec![]);
    let start = Pos2::new(300.0, 200.0);
    let end = Pos2::new(500.0, 400.0);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    frame(&ctx, &mut picker, vec![Event::PointerMoved(end)]);
    frame(&ctx, &mut picker, vec![pointer(end, false)]);
    assert_eq!(picker.selection.unwrap().min, Pos2::new(100.0, 100.0));
    assert_eq!(picker.selection.unwrap().max, end);
}

#[test]
fn dragging_tracks_each_pointer_event_without_parent_updates() {
    let ctx = egui::Context::default();
    let mut picker = picker();
    frame(&ctx, &mut picker, vec![]);
    let start = Pos2::new(600.0, 300.0);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(start), pointer(start, true)],
    );
    for i in 1..=200 {
        let point = start + Vec2::new(i as f32, i as f32);
        frame(&ctx, &mut picker, vec![Event::PointerMoved(point)]);
        assert_eq!(picker.selection.unwrap().max, point);
    }
    let end = Pos2::new(800.0, 500.0);
    frame(&ctx, &mut picker, vec![pointer(end, false)]);
    assert!(picker.drag.is_none());
}
