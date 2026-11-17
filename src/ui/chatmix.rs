use eframe::egui::{ComboBox, Ui};

use crate::mixer::{Channel, Config, Pair, Snapshot};

pub(super) fn controls(ui: &mut Ui, state: &mut Snapshot) -> Result<(), String> {
    ui.separator();
    let mut enabled = state.config.pair.is_some();
    ui.checkbox(&mut enabled, "Фізичне коліщатко ChatMix");
    let previous = state.config.pair;
    let mut pair = previous.unwrap_or(Pair {
        left: Channel::Game,
        right: Channel::Media,
    });
    if enabled {
        ui.horizontal_wrapped(|ui| {
            let prior = pair;
            ComboBox::from_id_salt("chatmix-left")
                .selected_text(pair.left.name())
                .show_ui(ui, |ui| {
                    for channel in Channel::ALL {
                        ui.selectable_value(&mut pair.left, channel, channel.name());
                    }
                });
            if prior.left != pair.left && pair.left == pair.right {
                pair.right = prior.left;
            }
            ui.label("↔");
            let prior = pair;
            ComboBox::from_id_salt("chatmix-right")
                .selected_text(pair.right.name())
                .show_ui(ui, |ui| {
                    for channel in Channel::ALL {
                        ui.selectable_value(&mut pair.right, channel, channel.name());
                    }
                });
            if prior.right != pair.right && pair.left == pair.right {
                pair.left = prior.right;
            }
        });
        if let Some([left, right]) = state.dial {
            ui.label(format!(
                "{} {left}% · {} {right}%",
                pair.left.name(),
                pair.right.name()
            ));
        } else {
            ui.label("Очікую позицію коліщатка з донгла.");
        }
    }
    let selection = enabled.then_some(pair);
    if selection != previous {
        Config::edit(|config| config.pair = selection)?;
        state.config.pair = selection;
    }
    Ok(())
}
