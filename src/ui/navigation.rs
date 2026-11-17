use eframe::egui::{Context, PointerButton};

use super::engine::{Engine, Page};

#[derive(Default)]
pub(super) struct Navigation {
    back: Vec<Page>,
    forward: Vec<Page>,
}

impl Engine {
    pub fn navigate(&mut self, page: Page) {
        if page != self.page {
            self.navigation.back.push(self.page);
            self.navigation.forward.clear();
            self.page = page;
        }
    }

    pub fn mouse_navigation(&mut self, ctx: &Context) {
        let (back, forward) = ctx.input(|input| {
            (
                input.pointer.button_pressed(PointerButton::Extra1),
                input.pointer.button_pressed(PointerButton::Extra2),
            )
        });
        if back && let Some(page) = self.navigation.back.pop() {
            self.navigation.forward.push(self.page);
            self.page = page;
        } else if forward && let Some(page) = self.navigation.forward.pop() {
            self.navigation.back.push(self.page);
            self.page = page;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, Modifiers, Pos2, RawInput};

    #[test]
    fn side_buttons_walk_the_visited_pages_and_new_navigation_clears_forward() {
        let context = Context::default();
        let mut engine = Engine::default();
        engine.navigate(Page::Mixer);
        engine.navigate(Page::Eq);
        let press = |engine: &mut Engine, button, pressed| {
            let mut output = context.run_ui(
                RawInput {
                    events: vec![Event::PointerButton {
                        pos: Pos2::ZERO,
                        button,
                        pressed,
                        modifiers: Modifiers::NONE,
                    }],
                    ..RawInput::default()
                },
                |ui| engine.mouse_navigation(ui.ctx()),
            );
            output.textures_delta.clear();
        };
        press(&mut engine, PointerButton::Extra1, true);
        assert!(engine.page == Page::Mixer);
        press(&mut engine, PointerButton::Extra1, false);
        press(&mut engine, PointerButton::Extra2, true);
        assert!(engine.page == Page::Eq);
        press(&mut engine, PointerButton::Extra2, false);
        press(&mut engine, PointerButton::Extra1, true);
        engine.navigate(Page::Settings);
        press(&mut engine, PointerButton::Extra2, true);
        assert!(engine.page == Page::Settings);
    }
}
