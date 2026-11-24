use std::{
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

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
    reading: bool,
    retry_read_at: Option<Instant>,
    error: Option<String>,
    path: String,
    search: String,
    only_favorites: bool,
    library: Option<Vec<eq::catalog::Entry>>,
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

    pub fn ensure_loaded(&mut self, ctx: &Context) {
        if self.busy() {
            return;
        }
        let now = Instant::now();
        if let Some(retry) = self.retry_read_at
            && retry > now
        {
            ctx.request_repaint_after(retry - now);
        } else if self.retry_read_at.is_some() || (self.active.is_none() && self.error.is_none()) {
            self.read(ctx);
        }
    }

    fn read(&mut self, ctx: &Context) {
        self.run(ctx, eq::current);
        self.reading = true;
    }

    fn run(
        &mut self,
        ctx: &Context,
        task: impl FnOnce() -> Result<Profile, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.request = Some(rx);
        self.error = None;
        self.retry_read_at = None;
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
        let was_reading = std::mem::take(&mut self.reading);
        self.retry_read_at = None;
        match result {
            Ok(profile) => {
                self.error = None;
                if let (Some(action), Some(editor)) = (action, self.editor.as_mut()) {
                    editor.compared(&action, &profile);
                } else {
                    let previous = self.active.clone().unwrap_or_else(|| profile.clone());
                    self.editor = Some(Editor::new(previous, profile.clone()));
                }
                self.active = Some(profile);
            }
            Err(error) => {
                if was_reading {
                    self.retry_read_at = Some(Instant::now() + Duration::from_secs(3));
                }
                self.error = Some(error);
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        prefs: &mut Preferences,
        device_busy: bool,
        control_error: Option<&str>,
    ) {
        let lang = prefs.language;
        if let Some(error) = control_error.or(self.error.as_deref()) {
            ui.colored_label(WARNING, error);
        }
        let enabled = !self.busy() && !device_busy && control_error.is_none();
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
                            .add_enabled(
                                enabled,
                                egui::Button::new(
                                    lang.text("Зчитати EQ гарнітури", "Read headset EQ"),
                                ),
                            )
                            .clicked()
                        {
                            self.read(ui.ctx());
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
                    self.read(ui.ctx());
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
        if ui
            .small_button(lang.text("Перечитати каталог", "Reload library"))
            .clicked()
        {
            self.library = None;
        }
        let needle = self.search.trim().to_lowercase();
        let library = self.library.get_or_insert_with(eq::catalog::load);
        let mut choose = None;
        let mut favorite_change = false;
        let mut visible = false;
        egui::ScrollArea::vertical()
            .id_salt("presets-library")
            .max_height(265.0)
            .show(ui, |ui| {
                for preset in library {
                    let favorite = prefs.favorites.contains(&preset.key);
                    if !preset.title.to_lowercase().contains(&needle)
                        || (self.only_favorites && !favorite)
                    {
                        continue;
                    }
                    visible = true;
                    ui.horizontal(|ui| {
                        let selected = preset
                            .profile
                            .as_ref()
                            .is_ok_and(|profile| self.active.as_ref() == Some(profile));
                        let mut row = ui
                            .add_enabled_ui(enabled && preset.profile.is_ok(), |ui| {
                                let width = (ui.available_width() - 30.0).max(1.0);
                                ui.add_sized(
                                    [width, ui.spacing().interact_size.y],
                                    egui::Button::new(&preset.title)
                                        .truncate()
                                        .selected(selected),
                                )
                            })
                            .inner
                            .on_hover_text(&preset.title);
                        if let Err(error) = &preset.profile {
                            row = row.on_disabled_hover_text(error);
                        }
                        if row.clicked()
                            && let Ok(profile) = &preset.profile
                        {
                            choose = Some(profile.clone());
                        }
                        if ui
                            .small_button(if favorite { "★" } else { "☆" })
                            .on_hover_text(lang.text("Обране", "Favorites"))
                            .clicked()
                        {
                            if favorite {
                                prefs.favorites.remove(&preset.key);
                            } else {
                                prefs.favorites.insert(preset.key.clone());
                            }
                            favorite_change = true;
                        }
                    });
                }
            });
        if !visible {
            ui.label(lang.text("Нічого не знайдено", "No matches"));
        }
        if favorite_change && let Err(error) = prefs.save() {
            self.error = Some(error);
        }
        if let Some(profile) = choose {
            self.run(ui.ctx(), move || eq::apply(&profile));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_failed_reads_are_retried_and_success_clears_the_error() {
        let mut page = EqPage::default();
        let mut finish = |result: Result<Profile, String>, reading: bool| {
            let (tx, rx) = mpsc::channel();
            tx.send(result).unwrap();
            page.request = Some(rx);
            page.reading = reading;
            page.poll();
            (
                page.retry_read_at,
                page.error.clone(),
                page.name().map(str::to_owned),
            )
        };
        let (retry, error, _) = finish(Err("write failed".into()), false);
        assert!(retry.is_none());
        assert_eq!(error.as_deref(), Some("write failed"));

        let (retry, error, _) = finish(Err("read failed".into()), true);
        assert!(retry.is_some_and(|t| t > Instant::now()));
        assert_eq!(error.as_deref(), Some("read failed"));

        let (retry, error, name) = finish(Profile::preset("Flat"), true);
        assert!(retry.is_none());
        assert!(error.is_none());
        assert_eq!(name.as_deref(), Some("Flat"));
    }
}
