//! Клиент игры: окно, рендер, ввод, звук, интерфейс.
//!
//! Клиент только читает состояние симуляции и превращает события в эффекты.
//! Любое изменение мира идёт через команды.
//!
//! Этот файл только собирает приложение Bevy: какие плагины подключены
//! и какие системы в каком расписании работают. Сама логика лежит
//! в соседних модулях, чтобы весь состав клиента читался в одном месте.

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

mod overlay;
mod scene;

/// Название в заголовке окна. Название игры ещё не выбрано (открытый
/// вопрос в docs/decisions.md), поэтому пока рабочее. На английском,
/// как все тексты для игрока (решение № 22).
const GAME_TITLE: &str = "RTS (working title)";

fn main() {
    // Версия правил симуляции видна в заголовке: по ней сразу понятно,
    // с какими реплеями совместима запущенная сборка.
    let window_title = format!("{GAME_TITLE}, sim v{}", sim::SIM_VERSION);

    // DefaultPlugins это стандартный набор Bevy: окно, рендер, ввод, ассеты.
    // Меняем в нём только настройки главного окна.
    let window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: window_title,
            ..default()
        }),
        ..default()
    };

    // Частота расписания FixedUpdate. Bevy копит прошедшее время и запускает
    // FixedUpdate столько раз, сколько тиков в него поместилось: при 60
    // кадрах в секунду это один тик на два кадра. `f64::from` превращает
    // целое в дробное без потери точности.
    let tick_time = Time::<Fixed>::from_hz(f64::from(sim::TICKS_PER_SECOND));

    App::new()
        .add_plugins(DefaultPlugins.set(window_plugin))
        // Считает FPS и складывает в DiagnosticsStore. Сам ничего не рисует.
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(tick_time)
        .init_resource::<overlay::TickCounter>()
        // Startup выполняется один раз перед первым кадром.
        .add_systems(Startup, (scene::setup, overlay::setup))
        // FixedUpdate: с постоянной частотой тиков.
        .add_systems(FixedUpdate, overlay::advance_tick)
        // Update: один раз на каждый кадр.
        .add_systems(Update, overlay::update_text)
        .run();
}
