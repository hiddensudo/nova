use eframe::egui::{Button, DragValue, Grid, Ui};

use crate::device::eq::Profile;

pub(super) struct Editor {
    original: Profile,
    draft: Profile,
}

pub(super) enum EditAction {
    Apply(Profile),
    Save(Profile),
    Close,
}

impl Editor {
    pub(super) fn new(original: Profile) -> Self {
        let mut draft = original.clone();
        draft.name = "Custom".into();
        draft.kind = 1;
        Self { original, draft }
    }

    pub(super) fn show(&mut self, ui: &mut Ui) -> Option<EditAction> {
        ui.heading("Редактор EQ");
        ui.horizontal(|ui| {
            ui.label("Назва");
            ui.text_edit_singleline(&mut self.draft.name);
        });
        Grid::new("eq-bands").striped(true).show(ui, |ui| {
            for heading in ["Смуга", "Гц", "дБ", "Q"] {
                ui.strong(heading);
            }
            ui.end_row();
            for (index, band) in self.draft.bands.iter_mut().enumerate() {
                ui.label((index + 1).to_string());
                ui.add(DragValue::new(&mut band.hz).range(20..=20_000));
                let mut db = f64::from(band.gain_tenths) / 10.0;
                if ui
                    .add(
                        DragValue::new(&mut db)
                            .range(-12.0..=12.0)
                            .speed(0.1)
                            .fixed_decimals(1),
                    )
                    .changed()
                {
                    band.gain_tenths = (db * 10.0).round() as i8;
                }
                let mut q = f64::from(band.q_thousandths) / 1000.0;
                if ui
                    .add(
                        DragValue::new(&mut q)
                            .range(0.2..=10.0)
                            .speed(0.01)
                            .fixed_decimals(3),
                    )
                    .changed()
                {
                    band.q_thousandths = (q * 1000.0).round() as u16;
                }
                ui.end_row();
            }
        });
        let validation = self.draft.validate();
        if let Err(error) = &validation {
            ui.colored_label(super::theme::WARNING, error);
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(validation.is_ok(), Button::new("Прослухати"))
                .clicked()
            {
                Some(EditAction::Apply(self.draft.clone()))
            } else if ui
                .add_enabled(validation.is_ok(), Button::new("Зберегти в гарнітурі"))
                .clicked()
            {
                Some(EditAction::Save(self.draft.clone()))
            } else if ui.button("Відновити попередній").clicked() {
                Some(EditAction::Apply(self.original.clone()))
            } else if ui.button("Закрити редактор").clicked() {
                Some(EditAction::Close)
            } else {
                None
            }
        })
        .inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_keeps_the_original_for_restore() {
        let original = Profile::preset("Focus").unwrap();
        let mut editor = Editor::new(original.clone());
        editor.draft.bands[0].gain_tenths = 10;
        assert_eq!(editor.original, original);
        assert_eq!(editor.draft.kind, 1);
        assert_ne!(editor.draft, original);
    }
}
