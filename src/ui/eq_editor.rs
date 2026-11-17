use eframe::egui::{Button, ComboBox, DragValue, RichText, Ui};

use crate::device::eq::{self, Profile};

use super::{
    eq_curve::Curve,
    preferences::Language,
    theme::{ACCENT, MUTED, SIDEBAR, WARNING},
};

pub(super) struct Editor {
    original: Profile,
    draft: Profile,
    curve: Curve,
    comparison: Comparison,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Comparison {
    Original,
    #[default]
    Draft,
}

pub(super) enum EditAction {
    CompareA(Profile),
    CompareB(Profile),
    Restore(Profile),
    Save(Profile),
}

impl Editor {
    pub fn new(original: Profile, draft: Profile) -> Self {
        Self {
            original,
            draft,
            curve: Curve::default(),
            comparison: Comparison::Draft,
        }
    }

    pub fn compared(&mut self, action: &EditAction, profile: &Profile) {
        self.comparison = if matches!(action, EditAction::CompareA(_)) {
            Comparison::Original
        } else {
            Comparison::Draft
        };
        if matches!(action, EditAction::Restore(_) | EditAction::Save(_)) {
            self.original = profile.clone();
            self.draft = profile.clone();
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        active: Option<&Profile>,
        lang: Language,
        enabled: bool,
    ) -> Option<EditAction> {
        let validation = self.draft.validate();
        let mut action = None;
        ui.horizontal(|ui| {
            let title = if self.comparison == Comparison::Original {
                &self.original.name
            } else {
                &self.draft.name
            };
            ui.strong(title);
            ui.with_layout(
                eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                |ui| {
                    if ui
                        .add_enabled(
                            enabled && validation.is_ok(),
                            Button::new("B").selected(self.comparison == Comparison::Draft),
                        )
                        .on_hover_text(lang.text("Прослухати поточну криву", "Listen to draft"))
                        .clicked()
                    {
                        action = Some(EditAction::CompareB(self.draft.clone()));
                    }
                    if ui
                        .add_enabled(
                            enabled,
                            Button::new("A").selected(self.comparison == Comparison::Original),
                        )
                        .on_hover_text(
                            lang.text("Прослухати попередню криву", "Listen to previous curve"),
                        )
                        .clicked()
                    {
                        action = Some(EditAction::CompareA(self.original.clone()));
                    }
                },
            );
        });
        let changed = if self.comparison == Comparison::Original {
            self.curve.show(ui, &mut self.original.bands, false)
        } else {
            self.curve.show(ui, &mut self.draft.bands, enabled)
        };
        ui.label(
            RichText::new(lang.text(
                "Розрахункова крива · Гц / дБ",
                "Calculated response · Hz / dB",
            ))
            .small()
            .color(MUTED),
        );
        let changes = ui
            .add_enabled_ui(enabled && self.comparison == Comparison::Draft, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(lang.text("Смуга", "Band"));
                    ComboBox::from_id_salt("eq-band")
                        .selected_text((self.curve.selected + 1).to_string())
                        .show_ui(ui, |ui| {
                            for index in 0..eq::BAND_COUNT {
                                ui.selectable_value(
                                    &mut self.curve.selected,
                                    index,
                                    (index + 1).to_string(),
                                );
                            }
                        });
                    let band = &mut self.draft.bands[self.curve.selected];
                    let frequency = ui.add(
                        DragValue::new(&mut band.hz)
                            .range(20..=20_000)
                            .suffix(" Hz"),
                    );
                    let mut gain = f32::from(band.gain_tenths) / 10.0;
                    let db = ui.add(
                        DragValue::new(&mut gain)
                            .range(-12.0..=12.0)
                            .speed(0.1)
                            .fixed_decimals(1)
                            .suffix(" dB"),
                    );
                    let mut q = f32::from(band.q_thousandths) / 1000.0;
                    let q_value = ui.add(
                        DragValue::new(&mut q)
                            .range(0.2..=10.0)
                            .speed(0.01)
                            .fixed_decimals(3)
                            .prefix("Q "),
                    );
                    if db.changed() {
                        band.gain_tenths = (gain * 10.0).round() as i8;
                    }
                    if q_value.changed() {
                        band.q_thousandths = (q * 1000.0).round() as u16;
                    }
                    frequency.changed() || db.changed() || q_value.changed()
                })
                .inner
            })
            .inner;
        if changed || changes {
            if eq::PRESETS.contains(&self.draft.name.as_str()) {
                self.draft.name = "Custom".into();
            }
            self.draft.kind = 1;
        }
        if let Err(error) = self.draft.validate() {
            ui.colored_label(WARNING, error);
        } else if active != Some(&self.draft) && self.comparison == Comparison::Draft {
            ui.label(
                RichText::new(lang.text(
                    "Чернетка · натисни B, щоб прослухати",
                    "Draft · press B to listen",
                ))
                .color(MUTED),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    enabled && self.draft != self.original,
                    Button::new(lang.text("Відновити", "Restore")),
                )
                .clicked()
            {
                action = Some(EditAction::Restore(self.original.clone()));
            }
            let save = Button::new(
                RichText::new(lang.text("Зберегти в гарнітурі", "Save to headset")).color(SIDEBAR),
            )
            .fill(ACCENT);
            if ui
                .add_enabled(enabled && self.draft.validate().is_ok(), save)
                .clicked()
            {
                action = Some(EditAction::Save(self.draft.clone()));
            }
        });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparing_curves_preserves_the_edit_until_a_restore() {
        let original = Profile::preset("Flat").unwrap();
        let draft = Profile::preset("Focus").unwrap();
        let mut editor = Editor::new(original.clone(), draft.clone());
        editor.compared(&EditAction::CompareA(original.clone()), &original);
        assert_eq!(editor.draft, draft);
        editor.compared(&EditAction::Restore(original.clone()), &original);
        assert_eq!(editor.draft, original);
    }
}
