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

## Апаратний EQ

Вкладка **EQ** показує активний профіль і перемикає чотири локальні пресети Flat, Bass Boost, Focus та Smiley. JSON-профілі Nova Control і сумісні пікові криві Sonar можна застосувати до EQ гарнітури. Частоти округлюються до 1 Гц, підсилення до 0,1 дБ, Q до 0,001. Ефекти Sonar поза параметричною кривою залишаються в Sonar.

```sh
nova-control eq list
nova-control eq get
nova-control eq select Focus
nova-control eq export ~/nova-eq.json
nova-control eq import ~/nova-eq.json
```

Зміни EQ перевіряються повторним читанням із донгла. Поточна крива лишається активною до наступної зміни або вимкнення. [Формат апаратного EQ Gen 2](https://github.com/loteran/Arctis-Sound-Manager/issues/146)

Редактор у вкладці **EQ** редагує частоту, підсилення й Q кожної з десяти пікових смуг. **Прослухати** застосовує чернетку в гарнітурі на поточну сесію. **Відновити попередній** повертає профіль, з яким було відкрито редактор. **Зберегти в гарнітурі** або `nova-control eq save` надсилає команду збереження поточних налаштувань.

## Мікшер PipeWire

У вкладці **Мікшер** увімкни **Game / Media / Chat**. Для кожного каналу є окрема гучність. Щоб призначити канал застосунку, запусти в ньому відтворення й вибери канал зі списку. Призначення та рівні зберігаються в `$XDG_CONFIG_HOME/nova-control/mixer.json` або `~/.config/nova-control/mixer.json`. Під час наступного запуску мікшера призначення застосовуються знову.

Потрібен `pactl` і запущений `pipewire-pulse`. На Debian та Ubuntu команда `pactl` є в пакунку `pulseaudio-utils`, на Arch — `libpulse`. Мікшер працює, поки відкритий Nova Control, або у терміналі з `nova-control mixer run`. Зупинка прибирає віртуальні канали.

У мікшері можна увімкнути **Фізичне коліщатко ChatMix** і вибрати будь-які два канали. Ліва позиція коліщатка відповідає першому каналу у списку. Третій канал і загальна гучність гарнітури зберігають власні рівні. Через CLI використовуй `nova-control mixer chatmix Game Media`. Поточну апаратну позицію читає `nova-control mixer dial`.

## Налаштування пристрою

Вкладка **Пристрій** показує заряд і стан заряджання, регулює рівень мікрофона, таймер автовимкнення та яскравість LED вимкненого мікрофона. Команди термінала `nova-control settings get`, `nova-control settings mic 6`, `nova-control settings sleep 30`, `nova-control settings led 2` і `nova-control settings save`.
