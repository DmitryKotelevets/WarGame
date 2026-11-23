# RTS (рабочее название)

Стратегия в реальном времени на Rust. Подробности в `docs/design.md`.

## Что здесь есть

```
CLAUDE.md            правила для агента
Cargo.toml           workspace
crates/sim           детерминированное ядро игры
crates/client        клиент на Bevy 0.20.1: окно, рендер, ввод
crates/tools         запуск без окна, позже MCP-сервер
docs/design.md       какая игра
docs/roadmap.md      вехи и шаги
docs/process.md      как идёт работа
docs/decisions.md    принятые и открытые решения
docs/ideas.md        отложенное
docs/stages/         спецификации вех и файлы шагов
```

## Проверка

Нужен стабильный Rust 1.97.1 или новее (`rustup` поставит его сам
по `rust-toolchain.toml`).

```sh
scripts/check.sh      # форматирование, clippy, тесты
cargo run -p tools
```

## Запуск клиента

```sh
cargo run -p client                              # обычная сборка
cargo run -p client --features dynamic_linking   # быстрая пересборка в разработке
```

Первая сборка Bevy занимает несколько минут и около 12 ГБ в `target`.

На Windows скрипт запускается из Git Bash.

## Перед первой сессией

```sh
git init
git add .
git commit -m "Скелет проекта"
```

## Первая сессия с агентом

Шаги 1.1 и 1.2 уже выполнены этим скелетом. Работа начинается с шага 1.3.

Текст первого запроса:

> Прочитай CLAUDE.md, docs/process.md, docs/roadmap.md
> и docs/stages/01-skeleton/spec.md. Начинаем шаг 1.3. Создай файл шага
> по шаблону, заполни «Цель», «План» и «Тесты» и остановись.

Дальше по `docs/process.md`: прочитать план, задать вопросы, подтвердить,
после реализации прочитать diff и итог.
