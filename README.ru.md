# Initium

[English version](README.md)

[Участие в проекте](CONTRIBUTING.ru.md) · [GPL-2.0-only](LICENSE)

Rust workspace для компонента ранней загрузки.

## Пакет для Arch Linux

Соберите пакет из архива исходников текущей рабочей копии Initium.

```bash
tar --transform='s,^,initium-0.1.0/,' -czf initium-0.1.0.tar.gz \
    .cargo Cargo.toml Cargo.lock crates systemd LICENSE \
    README{,.ru}.md CONTRIBUTING{,.ru}.md CODE_QUALITY{,.ru}.md COMMIT_CONVENTIONS{,.ru}.md
makepkg -si
```

Пакет устанавливает
`initium`, `initium-gui-preview`, `initium-shutdown`, юниты выключения,
документацию. Настройка и включение юнитов описаны ниже.
Конфиги устройства и initramfs входят в образ системы. При сборке
initramfs включите в него бинарник GUI и его динамические библиотеки.

## Крейты workspace

- [`initium`](crates/initium/)
- [`initium-config`](crates/initium-config/)
- [`initium-flow`](crates/initium-flow/)
- [`initium-gui`](crates/initium-gui/)

## Сборка GUI

`initium-gui` собирает встроенные исходники LVGL на C. Для сборки нужны
компилятор C и libclang. При кросс-компиляции нужен компилятор C для целевой
архитектуры.

## Подключение дисплея

Вызовите `initium_gui::initialize()` перед подключением через `DisplayPort`.
Для Linux framebuffer используйте `Fbdev::new("/dev/fb0")`, для DRM/KMS —
`DrmKms::new("/dev/dri/card0", None)`. Чтобы выбрать конкретный выход,
передайте идентификатор коннектора вместо `None`. Храните возвращённый
`DisplayConnection`, пока GUI использует дисплей.

Адаптеры используют драйверы LVGL для Linux. Для их сборки нужны заголовки
и библиотека libdrm. Если заголовки лежат вне системных путей поиска,
укажите в `C_INCLUDE_PATH` каталоги с `xf86drm.h` и `drm.h`. Для статической
сборки под musl нужны заголовки libdrm и `libdrm.a` для целевой архитектуры.
Путь к каталогу с архивом передайте Rust через `-L native=<каталог>`.

Для выбора адаптера в TOML добавьте необязательную секцию `[ui.display]`
и после инициализации передайте её в `initium_gui::connect_display`.
Для `backend = "fbdev"` указывается путь к framebuffer. Для
`backend = "drm-kms"` указывается путь к карте и, при необходимости,
`connector_id`.

```toml
[ui.display]
backend = "drm-kms"
path = "/dev/dri/card0"
```

## Конфигурация заставки

В конфиге заставки задаются подпись и последовательности кадров BGRA
с RLE-сжатием для вступления и цикла. При запуске GUI должен считать кадр,
оставленный загрузчиком ABL, прямо из framebuffer до первого обновления.
Этот кадр служит основой для закрашивания и остаётся в памяти. Вызовите
`BootloaderFrame::capture` с путём к устройству Linux framebuffer перед
подключением дисплея, затем передайте кадр в
`DisplayConnection::show_bootloader_frame`.

Секция `[ui.splash.bootloader_frame]` включает считывание framebuffer.
У необязательной области закрашивания координаты заданы в пикселях от
верхнего левого угла, а цвет — тремя компонентами RGB. GUI масштабирует
прямоугольник под размер кадра.

```toml
[ui.splash]
brand_text = "Example Linux"
intro_frames = "/assets/intro.frames"
loop_frames = "/assets/loop.frames"

[ui.splash.bootloader_frame.cover]
reference_width = 1920
reference_height = 1200
x = 760
y = 930
width = 400
height = 96
rgb = [0, 0, 0]
```

Если кадр ABL находится в области памяти, зарезервированной в Device Tree,
добавьте секцию `[ui.splash.bootloader_frame.reserved_memory]`. Initium ищет
узел в `reserved-memory` текущего Device Tree и считывает плотный кадр
XRGB8888 через `/dev/mem`. При отсутствии секции используется устройство
framebuffer, переданное при вызове.

```toml
[ui.splash.bootloader_frame.reserved_memory]
node = "memory@88000000"
width = 1920
height = 1200
```

## Проверка кадра ABL

Соберите для целевой архитектуры командой
`cargo build -p initium-gui --bin initium-gui-preview`. В ранней загрузке
вызовите `initium-gui-preview <config.toml>`, когда появится настроенное
устройство DRM или framebuffer. Утилита считывает кадр перед открытием
дисплея, применяет закрашивание и показывает результат пять секунд.

Путь к framebuffer для захвата задаёт `capture_device` в
`[ui.splash.bootloader_frame]`. При отсутствии пути и области резервной
памяти для DRM используется `/dev/fb0`. Вступление и цикл можно загрузить из файлов
`.frames` в initramfs. Секция `[ui.splash.animation]` задаёт размеры
кадров и частоты воспроизведения RLE-потоков. Параметр `--hold`
удерживает дисплей DRM и продолжает цикл после передачи управления.

Таблица финальной анимации задаёт длительность сближения и схлопывания,
радиусы в пикселях кадра и веса RGB для поиска движущейся части рисунка.
Если таблица отсутствует, финальная анимация пропускается.

```toml
[ui.splash.animation.outro]
merge_ms = 360
collapse_ms = 240
core_radius_px = 80
region_radius_px = 150
satellite_rgb_weights = [0, 1, 0]
```

## Конфигурация меню

Чтобы включить интерактивное меню, укажите путь evdev тачскрина в
`ui.touch_path`. Меню открывается и закрывается двойным касанием центра
анимированного логотипа. Перед появлением центрального ряда плиток
заставка поднимается.

- GUI сам разбирает и отображает журнал загрузки.
- Панель оборудования отображает устройства по их описаниям в конфиге.
- Меню выключения задаётся упорядоченным списком пунктов. У каждого пункта
  есть иконка, название и команда.
- Переключатель userspace показывает названия и иконки в порядке списка. Пустой или отсутствующий список скрывает переключатель.

Панель журнала при открытии считывает `/dev/kmsg`. Она показывает до
192 последних записей с временем и выделяет предупреждения и ошибки.
При открытии панель прокручивается к последней записи. Для открытия
используйте `DisplayConnection::show_boot_log`.

Preview может читать состояние загрузки из `ui.boot_state_dir`.
Переключатель userspace скрывается, когда системный корень готов и
последняя пауза загрузки закрыта, либо при получении стадии перехода.
При повторном открытии меню переключатель остаётся скрытым.

```toml
[[ui.userspace]]
name = "Linux"
icon = "/assets/linux.png"

[[ui.userspace]]
name = "Recovery"
icon = "/assets/recovery.png"
```

Иконки userspace — файлы PNG. Передайте `config.ui.userspace` в
`DisplayConnection::show_userspace_selector` и храните возвращённый объект,
пока переключатель виден. Для вывода кадра вызовите
`DisplayConnection::refresh`.

Передайте список в `DisplayConnection::show_power_menu`. После нажатия
вызывающий код может выполнить команду через `PowerMenu::run_pending_command`.
Аргументы передаются программе напрямую.

```toml
[[ui.power_menu]]
name = "Reboot"
icon = "/assets/reboot.png"
command = { program = "/usr/bin/systemctl", args = ["reboot"] }
```

## Сценарий загрузки

`initium <config.toml>` проверяет весь сценарий и выполняет шаги по порядку.
У каждого шага уникальный `id` и тип зарегистрированной операции `kind`.
При ошибке сценарий останавливается. Встроенная операция `command`
запускает программу с указанным списком аргументов.

Операция `mount` монтирует файловую систему напрямую. Целевой каталог
должен существовать. В `options` можно передать параметры файловой системы,
а в `flags` — флаги монтирования, например `read-only`.

```toml
[[steps]]
id = "mount-proc"
kind = "mount"
source = "proc"
target = "/proc"
filesystem = "proc"
flags = ["no-suid", "no-dev", "no-exec"]
```

Каждый пункт `[[ui.hardware]]` описывает устройство и условие готовности.
Необязательное условие `detected` позволяет отличить обнаруженное устройство
от ожидающего инициализации. Условие проверяет наличие пути, сравнивает
содержимое файла с `equals` после удаления пробелов по краям или ищет
совпадение с шаблоном `glob`. Устройство может иметь только условие
обнаружения, если отдельной проверки готовности нет.
`HardwarePanel::update` перечитывает указанные пути.

```toml
[[ui.hardware]]
name = "Battery"
ready = { path = "/sys/class/power_supply/mm8013/present", equals = "1" }
```

## Общая тема

Экран загрузки и заставка выключения читают один файл `/etc/initium/theme.toml`.
Установите его в initramfs и в основной системе.

```toml
background_rgb = 0x000000
accent_rgb = 0xf0c16a
```

Эти же цвета используются по умолчанию. Тема задаёт фон после передачи
экрана от ABL, цвет подписи и акцентов меню загрузки, фон и цвет надписей
при выключении. Кадры анимации сохраняют свою палитру.

## Заставка выключения

Соберите `cargo build --release -p initium-gui --bin initium-shutdown` и
установите бинарник в `/usr/lib/initium/initium-shutdown`. Установите юниты
из [systemd/](systemd/) и конфиг устройства в `/etc/initium/shutdown.toml`.
Включите `initium-shutdown@poweroff.service` и
`initium-shutdown@reboot.service`. Для устройства с заданным путём к
яркости подсветки включите `initium-shutdown-blank.service`.

Initium ждёт завершения графических сеансов и освобождения DRM до двух
секунд. Заставка использует активный режим DRM, а для неактивного выхода
можно задать запасной режим. Кадр остаётся на экране и при наличии
настройки копируется в буфер консоли. Лимит службы заставки составляет
три секунды. Служба подсветки срабатывает после `final.target`.

По умолчанию Initium рисует текстовую подпись. Пути `poweroff_caption`
и `reboot_caption` в `[shutdown]` задают изображения в формате BGRA с
непредумноженным альфа-каналом. Их размеры задаёт `caption_size`.
Силуэты надписей окрашиваются акцентом общей темы. Дисплей можно выбрать
через `connector_id_file`, а место графического сеанса через `seat`
(по умолчанию `seat0`).
