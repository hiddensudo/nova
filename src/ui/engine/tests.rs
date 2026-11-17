use super::*;
use eframe::egui::{self, Event, Modifiers, PointerButton, Rect, Shape, pos2, vec2};

use crate::ui::theme::configure;

fn visible_text(shape: &Shape, clip: Rect) -> Vec<String> {
    match shape {
        Shape::Text(text) if clip.expand(1.0).contains_rect(text.visual_bounding_rect()) => {
            vec![text.galley.text().to_owned()]
        }
        Shape::Vec(shapes) => shapes
            .iter()
            .flat_map(|shape| visible_text(shape, clip))
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn four_slider_levels_fit_without_a_refresh_button() {
    let ctx = Context::default();
    configure(&ctx);
    let mut app = Engine {
        level: Some(Sidetone::Medium),
        ..Default::default()
    };
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 400.0))),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();
    let labels: Vec<_> = output
        .shapes
        .iter()
        .flat_map(|shape| visible_text(&shape.shape, shape.clip_rect))
        .collect();
    for level in Sidetone::ALL {
        assert!(
            labels.iter().any(|label| label == level.label()),
            "{labels:?}"
        );
    }
    assert!(!labels.iter().any(|label| label == "Оновити"));
    for tab in ["Мікрофон", "EQ", "Мікшер", "Пристрій"] {
        assert!(labels.iter().any(|label| label == tab), "{labels:?}");
    }
}

#[test]
fn dragging_previews_and_release_queues_only_the_final_level() {
    let ctx = Context::default();
    let now = Instant::now();
    let mut app = Engine {
        level: Some(Sidetone::Off),
        ..Default::default()
    };
    let rail = frame_for_drag(&ctx, &mut app, vec![]);
    let point = |position| pos2(egui::lerp(rail.x_range(), position), rail.center().y);
    let click = |position, pressed| Event::PointerButton {
        pos: point(position),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame_for_drag(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(point(0.01)), click(0.01, true)],
    );
    frame_for_drag(&ctx, &mut app, vec![Event::PointerMoved(point(0.34))]);
    assert_eq!(
        app.slider_selection,
        Some(SliderSelection::Dragging(Sidetone::Low))
    );
    assert!(app.next_operation(now).is_none());
    frame_for_drag(&ctx, &mut app, vec![Event::PointerMoved(point(0.99))]);
    frame_for_drag(&ctx, &mut app, vec![click(0.99, false)]);
    assert_eq!(
        app.slider_selection,
        Some(SliderSelection::Queued(Sidetone::High))
    );
    assert!(matches!(
        app.next_operation(now),
        Some(Operation::Set(Sidetone::High))
    ));
    assert!(matches!(app.next_operation(now), Some(Operation::Read)));
}

#[test]
fn reads_on_schedule_and_runs_queued_write_after_pending_read() {
    let now = Instant::now();
    let deadline = now + READ_INTERVAL;
    let mut app = Engine::default();
    assert!(matches!(app.next_operation(now), Some(Operation::Read)));

    app.next_read = Some(deadline);
    assert!(app.next_operation(now).is_none());
    assert!(matches!(
        app.next_operation(deadline),
        Some(Operation::Read)
    ));

    let (_sender, result) = mpsc::channel();
    app.request = Some(Request {
        operation: Operation::Read,
        result,
    });
    app.slider_selection = Some(SliderSelection::Queued(Sidetone::High));
    assert!(app.next_operation(deadline).is_none());
    app.request = None;
    assert!(matches!(
        app.next_operation(deadline),
        Some(Operation::Set(Sidetone::High))
    ));
}

fn frame_for_drag(ctx: &Context, app: &mut Engine, events: Vec<Event>) -> Rect {
    let mut rect = Rect::NOTHING;
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 400.0))),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        rect = app.sidetone_slider(ui).rect;
    });
    output.textures_delta.clear();
    rect
}
