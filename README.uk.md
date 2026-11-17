# Nova Control

[English](README.md)

Нативний застосунок для SteelSeries Arctis Nova 7 Wireless Gen 2 через USB-донгл `1038:227e`. Слайдер Mic Sidetone має чотири апаратні позиції **Off, Low, Medium і High**. Панель зчитує рівень автоматично. Зміна застосовується після відпускання повзунка. Кнопка **Зберегти в гарнітурі** або `nova-control sidetone save` записує поточні налаштування пристрою.

## Встановлення й оновлення

Розпакуй архів збірки й запусти інсталятор у його папці.

```sh
python3 install.py
```

Після цього **Nova Control** доступний у меню застосунків і пошуку KDE. Для оновлення виконай ту саму команду з новішого архіву та відкрий застосунок знову, якщо вікно старої версії ще працює. Встановлена копія в `~/.local/bin/nova-control` працює незалежно від архіву. Мінімальну версію glibc конкретної збірки вказано в `BUILD.txt`.

Щоб видалити застосунок, прибери `~/.local/bin/nova-control` і `nova-control.desktop` із `~/.local/share/applications` або з каталогу `applications` у своєму `XDG_DATA_HOME`.

## Збірка з коду

Потрібні Rust 1.95+, Python 3.11+, C-компілятор та pkg-config. Пакунки для Debian і Ubuntu перелічено в [README](README.md#build-from-source). У папці з кодом виконай `python3 install.py` для збірки й встановлення. Ця команда також оновлює встановлену копію після `git pull --ff-only` або розпакування новішого коду.

- `cargo build --release --locked` збирає без встановлення.
- `cargo test --locked` перевіряє Rust-код без відкриття вікна.
- `python3 scripts/package.py` створює готовий Linux-архів, архів коду й контрольні суми в `dist/`.

Додаткові команди і параметри пакування є в [README](README.md).

## Доступ до донгла

Встанови правило udev, якщо застосунок повідомляє про відсутність прав доступу до донгла.

```sh
sudo install -m 0644 packaging/linux/70-nova-control.rules /etc/udev/rules.d/70-nova-control.rules
sudo udevadm control --reload-rules
```

Після цього перепідключи донгл. Застосунок запускається від звичайного користувача.
