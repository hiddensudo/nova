use std::{path::PathBuf, sync::mpsc, thread};

use eframe::egui::{self, Context, Ui};

use crate::device::eq::{self, Profile};

use super::{
    eq_editor::{EditAction, Editor},
    preferences::Preferences,
    theme::{WARNING, card},
};

#[derive(Default)]
pub(super) struct EqPage {
    active: Option<Profile>,
    request: Option<mpsc::Receiver<Result<Profile, String>>>,
    action: Option<EditAction>,
    error: Option<String>,
    path: String,
    search: String,
    only_favorites: bool,
    editor: Option<Editor>,
}

impl EqPage {
    #[cfg(test)]
    pub fn fixture(profile: Profile) -> Self {
        Self {
            editor: Some(Editor::new(profile.clone(), profile.clone())),
            active: Some(profile),
            ..Self::default()
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.active.as_ref().map(|profile| profile.name.as_str())
    }

    pub fn ensure_loaded(&mut self, ctx: &Context) -> bool {
        if self.active.is_none() && self.error.is_none() && !self.busy() {
            self.run(ctx, eq::current);
            true
        } else {
            false
        }
    }

    fn run(
        &mut self,
        ctx: &Context,
        task: impl FnOnce() -> Result<Profile, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.request = Some(rx);
        self.error = None;
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(task());
            ctx.request_repaint();
        });
    }

    pub fn poll(&mut self) {
        let Some(request) = &self.request else { return };
        let result = match request.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("EQ request interrupted.".into()),
        };
        self.request = None;
        let action = self.action.take();
        match result {
            Ok(profile) => {
                if let (Some(action), Some(editor)) = (action, self.editor.as_mut()) {
                    editor.compared(&action, &profile);
                } else {
                    let previous = self.active.clone().unwrap_or_else(|| profile.clone());
                    self.editor = Some(Editor::new(previous, profile.clone()));
                }
                self.active = Some(profile);
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn show(&mut self, ui: &mut Ui, prefs: &mut Preferences, device_busy: bool) {
        let lang = prefs.language;
        if !device_busy {
            self.ensure_loaded(ui.ctx());
        }
        if let Some(error) = &self.error {
            ui.colored_label(WARNING, error);
        }
        let enabled = !self.busy() && !device_busy;
        let mut panel_height = 330.0;
        ui.horizontal_top(|ui| {
            let width = ui.available_width();
            let catalog_width = (width * 0.24).clamp(148.0, 201.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width - catalog_width - ui.spacing().item_spacing.x, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let panel = card(ui, |ui| {
                        if let Some(editor) = &mut self.editor {
                            if let Some(action) =
                                editor.show(ui, self.active.as_ref(), lang, enabled)
                            {
                                self.edit(ui.ctx(), action);
                            }
                        } else if self.busy() || device_busy {
                            ui.spinner();
                        } else if ui
                            .button(lang.text("Зчитати EQ гарнітури", "Read headset EQ"))
                            .clicked()
                        {
                            self.run(ui.ctx(), eq::current);
                        }
                    });
                    panel_height = panel.response.rect.height() - 26.0;
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(catalog_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    card(ui, |ui| {
                        ui.set_min_height(panel_height);
                        self.catalog(ui, prefs, enabled);
                    });
                },
            );
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            ui.strong(lang.text("Файли профілів", "Profile files"));
            ui.add_space(5.0);
            let path_width = (ui.available_width() - 235.0).max(125.0);
            ui.horizontal(|ui| {
                ui.label(lang.text("Імпорт JSON", "Import JSON"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.path)
                        .desired_width(path_width)
                        .hint_text(lang.text("Шлях до профілю", "Path to profile")),
                );
                if ui
                    .add_enabled(
                        enabled && !self.path.trim().is_empty(),
                        egui::Button::new(lang.text("Імпортувати", "Import")),
                    )
                    .clicked()
                {
                    let path = PathBuf::from(self.path.trim());
                    self.run(ui.ctx(), move || eq::apply(&eq::from_file(&path)?));
                }
            });
            ui.horizontal_wrapped(|ui| {
                if let Some(active) = &self.active {
                    ui.label(format!(
                        "{} · {}",
                        lang.text("На гарнітурі", "On headset"),
                        active.name
                    ));
                }
                if ui
                    .add_enabled(
                        enabled,
                        egui::Button::new(lang.text("Зчитати профіль", "Read profile")),
                    )
                    .clicked()
                {
                    self.run(ui.ctx(), eq::current);
                }
            });
        });
    }

    fn catalog(&mut self, ui: &mut Ui, prefs: &mut Preferences, enabled: bool) {
        let lang = prefs.language;
        ui.strong(lang.text("Пресети", "Presets"));
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(lang.text("Пошук пресетів", "Search presets")),
        );
        super::controls::toggle(
            ui,
            &mut self.only_favorites,
            lang.text("Обране", "Favorites"),
        );
        let needle = self.search.trim().to_ascii_lowercase();
        let mut visible = false;
        for preset in eq::PRESETS {
            let favorite = prefs.favorites.contains(preset);
            if !preset.to_ascii_lowercase().contains(&needle) || (self.only_favorites && !favorite)
            {
                continue;
            }
            visible = true;
            ui.horizontal(|ui| {
                let selected = self.name() == Some(preset);
                if ui
                    .add_enabled(enabled, egui::Button::new(preset).selected(selected))
                    .clicked()
                {
                    self.run(ui.ctx(), move || eq::apply(&Profile::preset(preset)?));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(if favorite { "★" } else { "☆" })
                        .on_hover_text(lang.text("Обране", "Favorites"))
                        .clicked()
                    {
                        if favorite {
                            prefs.favorites.remove(preset);
                        } else {
                            prefs.favorites.insert(preset.into());
                        }
                        if let Err(error) = prefs.save() {
                            self.error = Some(error);
                        }
                    }
                });
            });
        }
        if !visible {
            ui.label(lang.text("Нічого не знайдено", "No matches"));
        }
    }

    fn edit(&mut self, ctx: &Context, action: EditAction) {
        let profile = match &action {
            EditAction::CompareA(profile)
            | EditAction::CompareB(profile)
            | EditAction::Restore(profile)
            | EditAction::Save(profile) => profile.clone(),
        };
        let save = matches!(action, EditAction::Save(_));
        self.action = Some(action);
        self.run(ctx, move || {
            let applied = eq::apply(&profile)?;
            if save {
                eq::save(&applied)
            } else {
                Ok(applied)
            }
        });
    }

    pub fn busy(&self) -> bool {
        self.request.is_some()
    }
}
