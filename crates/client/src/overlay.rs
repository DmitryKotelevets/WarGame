//! Оверлей в углу экрана: FPS и номер тика.
//!
//! Нужен, чтобы на глаз видеть две разные частоты: кадры рисуются так
//! часто, как успевает экран, а тики идут ровно `sim::TICKS_PER_SECOND`
//! раз в секунду. Настоящей симуляции ещё нет, поэтому тик здесь это
//! временный счётчик в клиенте. В вехе 2 его заменит номер тика из мира.
//!
//! Тексты на английском (решение № 22 в docs/decisions.md).

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::text::FontSize;

/// Отступ текста от левого и верхнего края окна в логических пикселях.
const MARGIN_PX: f32 = 8.0;

/// Размер шрифта в логических пикселях: читается, но не закрывает сцену.
const FONT_SIZE_PX: f32 = 18.0;

/// Временный счётчик тиков. Ресурс в Bevy это данные в одном экземпляре
/// на всё приложение, системы получают его по типу аргумента.
#[derive(Resource, Default)]
pub struct TickCounter {
    pub tick: u64,
}

/// Метка на тексте оверлея. Данных в ней нет: она нужна, чтобы система
/// обновления находила именно этот текст среди всех текстов интерфейса.
#[derive(Component)]
pub struct OverlayText;

/// Собирает текст оверлея. Обычная функция без Bevy, чтобы её можно было
/// проверить тестом без окна.
///
/// `fps` равен `None`, пока Bevy не накопил измерений (первые кадры).
/// Тогда вместо числа стоят прочерки: ноль выглядел бы как настоящий замер.
pub fn overlay_text(fps: Option<f64>, tick: u64) -> String {
    match fps {
        // `{fps:.0}` печатает число без дробной части с округлением.
        Some(fps) => format!("FPS: {fps:.0}\nTick: {tick}"),
        None => format!("FPS: --\nTick: {tick}"),
    }
}

/// Создаёт текст в левом верхнем углу. Выполняется один раз при запуске.
pub fn setup(mut commands: Commands) {
    commands.spawn((
        OverlayText,
        Text::new(overlay_text(None, 0)),
        // Шрифт не указан: берётся встроенный в Bevy (урезанный FiraMono,
        // только латиница). Он моноширинный, поэтому цифры не «прыгают».
        TextFont {
            font_size: FontSize::Px(FONT_SIZE_PX),
            ..default()
        },
        TextColor(Color::WHITE),
        // Absolute: положение задаётся отступами от краёв окна, поэтому
        // текст остаётся в углу при любом размере окна.
        Node {
            position_type: PositionType::Absolute,
            left: px(MARGIN_PX),
            top: px(MARGIN_PX),
            ..default()
        },
    ));
}

/// Прибавляет один тик. Работает в расписании `FixedUpdate`, которое Bevy
/// запускает с постоянной частотой независимо от частоты кадров.
pub fn advance_tick(mut counter: ResMut<TickCounter>) {
    counter.tick += 1;
}

/// Каждый кадр записывает в текст свежие FPS и номер тика.
///
/// `Single` означает «ровно один объект с такими компонентами». Если их
/// окажется ноль или два, Bevy сообщит об ошибке, а не промолчит.
pub fn update_text(
    diagnostics: Res<DiagnosticsStore>,
    counter: Res<TickCounter>,
    mut text: Single<&mut Text, With<OverlayText>>,
) {
    // Сглаженное значение: среднее за последние кадры с большим весом
    // у недавних. Мгновенное значение мелькало бы и не читалось.
    let fps = match diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        Some(diagnostic) => diagnostic.smoothed(),
        None => None,
    };

    let new_text = overlay_text(fps, counter.tick);

    // Пишем только при изменении: любая запись в `Text` заставляет Bevy
    // заново раскладывать буквы, а тик меняется реже, чем кадр.
    if text.0 != new_text {
        text.0 = new_text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_rounded_fps_and_tick() {
        assert_eq!(overlay_text(Some(59.6), 123), "FPS: 60\nTick: 123");
    }

    #[test]
    fn shows_dashes_until_fps_is_measured() {
        assert_eq!(overlay_text(None, 0), "FPS: --\nTick: 0");
    }
}
