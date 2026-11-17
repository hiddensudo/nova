use ashpd::desktop::{
    Session,
    global_shortcuts::{GlobalShortcuts, NewShortcut},
};
use futures_lite::{StreamExt, future};

use super::{Action, Event, Language, Signal};

const ID: &str = "cycle-sidetone";

pub(super) fn register(signal: Signal, cancel: async_channel::Receiver<()>, lang: Language) {
    let result = future::block_on(async {
        let portal = GlobalShortcuts::new()
            .await
            .map_err(|error| error.to_string())?;
        let session = portal
            .create_session()
            .await
            .map_err(|error| error.to_string())?;
        let result = future::race(listen(&portal, &session, &signal, lang), async {
            let _ = cancel.recv().await;
            Ok(())
        })
        .await;
        let _ = session.close().await;
        result
    });
    if let Err(error) = result
        && !cancel.is_closed()
    {
        signal.send(Event::ShortcutError(error));
    }
}

async fn listen(
    portal: &GlobalShortcuts<'_>,
    session: &Session<'_, GlobalShortcuts<'_>>,
    signal: &Signal,
    language: Language,
) -> Result<(), String> {
    let path = serde_json::to_value(session).map_err(|error| error.to_string())?;
    let path = path
        .as_str()
        .ok_or("The portal returned an invalid session path.")?;
    let proposed = NewShortcut::new(
        ID,
        language.text("Наступний рівень Sidetone", "Next Sidetone level"),
    )
    .preferred_trigger("ALT+SHIFT+s");
    let request = portal
        .bind_shortcuts(session, &[proposed], None)
        .await
        .map_err(|error| error.to_string())?;
    let bound = request.response().map_err(|error| error.to_string())?;
    let trigger = bound
        .shortcuts()
        .iter()
        .find(|shortcut| shortcut.id() == ID)
        .ok_or_else(|| {
            language
                .text(
                    "Системну клавішу не призначено.",
                    "The desktop did not assign a shortcut.",
                )
                .to_owned()
        })?
        .trigger_description()
        .to_owned();
    let mut activated = std::pin::pin!(
        portal
            .receive_activated()
            .await
            .map_err(|error| error.to_string())?
    );
    let mut closed = std::pin::pin!(
        session
            .receive_closed()
            .await
            .map_err(|error| error.to_string())?
    );
    signal.send(Event::ShortcutReady(trigger));
    future::race(
        async {
            while let Some(event) = activated.next().await {
                if event.session_handle().as_str() == path && event.shortcut_id() == ID {
                    signal.send(Event::Action(Action::Cycle));
                }
            }
        },
        async {
            let _ = closed.next().await;
        },
    )
    .await;
    Err(language
        .text(
            "Сесію глобальних клавіш завершено.",
            "The global shortcuts session ended.",
        )
        .to_owned())
}
