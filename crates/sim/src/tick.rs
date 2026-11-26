//! Номер тика симуляции.
//!
//! Время в симуляции идёт не секундами, а шагами: один вызов
//! `World::step()` это один тик, `TICKS_PER_SECOND` тиков это секунда.
//! Номер тика получил свой тип, чтобы его нельзя было спутать с индексом
//! или количеством чего-нибудь: компилятор не даст передать одно вместо
//! другого.

/// Номер тика. Поле закрыто: менять номер может только `next`, поэтому
/// время не может пойти назад или перескочить.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Tick(u32);

impl Tick {
    /// Начало игры: ни одного шага ещё не сделано.
    pub const ZERO: Tick = Tick(0);

    /// Тик с заданным номером: для тестов, сценариев и чтения реплея.
    pub fn from_raw(raw: u32) -> Tick {
        Tick(raw)
    }

    /// Номер тика обычным числом: для хеша, реплея и показа на экране.
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Следующий тик.
    pub fn next(self) -> Tick {
        match self.0.checked_add(1) {
            Some(raw) => Tick(raw),
            // При 30 тиках в секунду это 4,5 года непрерывной игры.
            // Завернуться в ноль нельзя: команды и реплей привязаны
            // к номерам тиков.
            None => panic!("Tick overflow"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_next() {
        assert_eq!(Tick::ZERO.raw(), 0);
        assert_eq!(Tick::ZERO.next().raw(), 1);
        assert_eq!(Tick::from_raw(41).next(), Tick::from_raw(42));
    }

    #[test]
    fn ticks_are_ordered() {
        assert!(Tick::ZERO < Tick::ZERO.next());
        assert!(Tick::from_raw(10) > Tick::from_raw(9));
        assert_eq!(Tick::from_raw(0), Tick::ZERO);
    }

    #[test]
    #[should_panic(expected = "Tick overflow")]
    fn next_after_the_last_tick_panics() {
        let _ = Tick::from_raw(u32::MAX).next();
    }
}
