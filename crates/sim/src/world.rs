//! Мир симуляции.
//!
//! `World` владеет всем состоянием игры. Если два мира равны поле в поле,
//! их будущее одинаково: на этом держатся реплеи и будущая сетевая игра.
//! Поэтому никакого состояния симуляции вне `World` быть не должно
//! (ни глобальных переменных, ни значений, спрятанных в системах).
//!
//! Поля открыты. Системы это обычные функции, которым нужны сразу
//! несколько полей мира, а клиент получает мир только по неизменяемой
//! ссылке `&World` и испортить его не может.
//!
//! Продвигает мир `World::step()` из `step.rs`.

use crate::tick::Tick;
use crate::unit_store::UnitStore;

/// Всё состояние симуляции.
#[derive(Default)]
pub struct World {
    /// Сколько шагов уже выполнено. Во время шага это номер тика,
    /// который сейчас считается.
    pub tick: Tick,
    /// Все юниты.
    pub units: UnitStore,
}

impl World {
    /// Мир в начале игры: тик 0, юнитов нет.
    pub fn new() -> World {
        World {
            tick: Tick::ZERO,
            units: UnitStore::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_world_is_empty_at_tick_zero() {
        let world = World::new();
        assert_eq!(world.tick, Tick::ZERO);
        assert!(world.units.is_empty());
    }
}
