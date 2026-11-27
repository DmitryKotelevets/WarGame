//! Генератор случайных чисел.
//!
//! Числа только выглядят случайными: это длинная запутанная
//! последовательность, которая полностью определена начальным числом
//! (сидом). Тот же сид даёт ту же последовательность на любой машине,
//! поэтому реплей с разбросом снарядов воспроизводится точно.
//!
//! Алгоритм называется SplitMix64 (автор Себастьяно Винья, общественное
//! достояние). Выбран за простоту: всё состояние это одно число `u64`,
//! годится любой сид, включая ноль. Последовательность повторится только
//! через 2^64 чисел.
//!
//! В отличие от `Fixed`, арифметика здесь заворачивается при переполнении
//! (`wrapping_add`, `wrapping_mul`). Это не ошибка, а часть алгоритма:
//! он так и задуман, по модулю 2^64. Заворачивание записано явно, поэтому
//! результат одинаков в отладочной и выпускной сборке.
//!
//! Единственный генератор игры лежит в `World`. Создавать другие
//! в системах нельзя: их состояние не попало бы в хеш мира.

/// Шаг счётчика: на столько состояние сдвигается перед каждым числом.
/// Это 2^64, делённое на золотое сечение; число нечётное, поэтому счётчик
/// обходит все 2^64 значений, прежде чем повториться.
const INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;

/// Множители перемешивания из описания алгоритма.
const MIX_MULTIPLIER_1: u64 = 0xbf58_476d_1ce4_e5b9;
const MIX_MULTIPLIER_2: u64 = 0x94d0_49bb_1331_11eb;

/// Генератор. Поле закрыто: состояние меняется только выдачей чисел.
/// `Copy` нужен, чтобы состояние можно было сохранить и сравнить; копия
/// продолжает ту же последовательность, что и оригинал.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Генератор с заданным сидом.
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    /// Текущее состояние: для хеша мира.
    pub fn state(self) -> u64 {
        self.state
    }

    /// Следующее случайное число на все 64 бита.
    pub fn next_u64(&mut self) -> u64 {
        // Состояние это простой счётчик с большим шагом.
        self.state = self.state.wrapping_add(INCREMENT);

        // Соседние значения счётчика похожи друг на друга, поэтому
        // наружу отдаётся не оно само, а его перемешанная копия:
        // сдвиг с «исключающим или» разносит старшие биты в младшие,
        // умножение разносит младшие в старшие.
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(MIX_MULTIPLIER_1);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(MIX_MULTIPLIER_2);
        mixed ^ (mixed >> 31)
    }

    /// Следующее случайное число на 32 бита: старшая половина `next_u64`.
    pub fn next_u32(&mut self) -> u32 {
        let high_half = self.next_u64() >> 32;
        match u32::try_from(high_half) {
            Ok(value) => value,
            // После сдвига на 32 от 64 бит остаётся ровно 32.
            Err(_) => panic!("Rng::next_u32: high half does not fit in u32"),
        }
    }

    /// Случайное целое от 0 до `bound - 1`. Тратит ровно одно число
    /// генератора.
    ///
    /// `next_u32` делит отрезок от 0 до 2^32 равномерно. Умножение
    /// на `bound` растягивает его до `bound * 2^32`, сдвиг на 32 бита
    /// делит обратно на 2^32: получается число от 0 до `bound - 1`.
    /// Если `bound` не степень двойки, некоторые значения выпадают чуть
    /// чаще других, на долю порядка `bound / 2^32`. Для игры это незаметно.
    pub fn next_below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            panic!("Rng::next_below: bound must be positive");
        }
        let scaled = u64::from(self.next_u32()) * u64::from(bound);
        match u32::try_from(scaled >> 32) {
            Ok(value) => value,
            // Произведение меньше `bound * 2^32`, после сдвига оно
            // меньше `bound`.
            Err(_) => panic!("Rng::next_below: result does not fit in u32"),
        }
    }
}

#[cfg(test)]
mod tests {
    // `Rng` назван явно, а не через `super::*`: в `proptest::prelude`
    // есть свой `Rng`, и из двух «звёздочек» компилятор выбрать не может.
    // Явное имя сильнее звёздочки.
    use super::Rng;
    use proptest::prelude::*;

    #[test]
    fn matches_reference_values() {
        // Эталонные числа SplitMix64. Если тест упал, генератор стал
        // другим и все старые реплеи разошлись.
        let mut rng = Rng::new(0);
        assert_eq!(rng.next_u64(), 0xe220a8397b1dcdaf);
        assert_eq!(rng.next_u64(), 0x6e789e6aa1b965f4);
        assert_eq!(rng.next_u64(), 0x06c45d188009454f);

        let mut rng = Rng::new(1234567);
        assert_eq!(rng.next_u64(), 6457827717110365317);
        assert_eq!(rng.next_u64(), 3203168211198807973);
        assert_eq!(rng.next_u64(), 9817491932198370423);
    }

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut first = Rng::new(42);
        let mut second = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(first.next_u64(), second.next_u64());
        }
    }

    #[test]
    fn different_seeds_give_different_numbers() {
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn copy_continues_the_same_sequence() {
        let mut original = Rng::new(7);
        original.next_u64();
        let state_before = original.state();

        let mut copy = original;
        assert_eq!(copy.state(), original.state());
        assert_eq!(copy.next_u64(), original.next_u64());
        assert_eq!(copy, original);
        // Состояние сдвинулось.
        assert_ne!(original.state(), state_before);
    }

    #[test]
    fn next_u32_is_the_high_half_of_next_u64() {
        let mut narrow = Rng::new(99);
        let mut wide = Rng::new(99);
        for _ in 0..100 {
            let expected = u32::try_from(wide.next_u64() >> 32).expect("осталось 32 бита");
            assert_eq!(narrow.next_u32(), expected);
        }
    }

    #[test]
    fn next_below_one_is_always_zero() {
        let mut rng = Rng::new(5);
        for _ in 0..100 {
            assert_eq!(rng.next_below(1), 0);
        }
    }

    #[test]
    fn next_below_six_is_roughly_uniform() {
        let mut rng = Rng::new(2026);
        let mut counts = [0u32; 6];
        for _ in 0..10_000 {
            let value = rng.next_below(6);
            counts[usize::try_from(value).expect("число меньше шести")] += 1;
        }
        // В среднем 1667 на значение. Сид фиксирован, поэтому тест
        // не может упасть случайно.
        for count in counts {
            assert!((1500..=1850).contains(&count), "counts: {counts:?}");
        }
    }

    #[test]
    #[should_panic(expected = "next_below: bound must be positive")]
    fn next_below_zero_panics() {
        let _ = Rng::new(0).next_below(0);
    }

    proptest! {
        #[test]
        fn next_below_stays_below_bound(seed in any::<u64>(), bound in 1u32..) {
            let mut rng = Rng::new(seed);
            for _ in 0..10 {
                prop_assert!(rng.next_below(bound) < bound);
            }
        }
    }
}
