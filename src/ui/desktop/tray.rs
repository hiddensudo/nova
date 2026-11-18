use ksni::{
    MenuItem,
    blocking::TrayMethods,
    menu::{RadioGroup, RadioItem, StandardItem, SubMenu},
};

use crate::device::Sidetone;

use super::{Action, Event, Language, Signal};

pub(super) struct Tray {
    signal: Signal,
    pub language: Language,
    pub level: Option<Sidetone>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "nova-control".into()
    }

    fn title(&self) -> String {
        "Nova Control".into()
    }

    fn icon_name(&self) -> String {
        "nova-control-symbolic".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        crate::ui::branding::tray_icons()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.signal.send(Event::Action(Action::Show));
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let lang = self.language;
        vec![
            StandardItem {
                label: lang
                    .text("Відкрити Nova Control", "Open Nova Control")
                    .into(),
                activate: Box::new(|tray: &mut Self| tray.signal.send(Event::Action(Action::Show))),
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Sidetone".into(),
                submenu: vec![
                    RadioGroup {
                        selected: self.level.map_or(usize::MAX, |level| level as usize),
                        select: Box::new(|tray: &mut Self, index| {
                            if let Some(&level) = Sidetone::ALL.get(index) {
                                tray.signal.send(Event::Action(Action::Sidetone(level)));
                            }
                        }),
                        options: Sidetone::ALL
                            .into_iter()
                            .map(|level| RadioItem {
                                label: level.label().into(),
                                ..Default::default()
                            })
                            .collect(),
                    }
                    .into(),
                ],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: lang.text("Вийти", "Quit").into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| tray.signal.send(Event::Action(Action::Quit))),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub(super) fn register(signal: Signal, language: Language) {
    let tray = Tray {
        signal: signal.clone(),
        language,
        level: None,
    };
    if let Ok(handle) = tray.spawn() {
        signal.send(Event::Tray(handle));
    }
}
