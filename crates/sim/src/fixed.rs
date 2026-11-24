//! Число с фиксированной точкой.
//!
//! Обычные дробные числа (`f32`, `f64`) могут давать разный результат
//! на разных процессорах, а симуляция обязана совпадать бит в бит: на этом
//! держатся реплеи и будущая сетевая игра. Поэтому дробь хранится как
//! целое число `raw`, а значение равно `raw / 2^32`. Вся арифметика
//! целочисленная и везде считается одинаково.
//!
//! Формат Q32.32: 64 бита, из них 32 на дробную часть (решение № 23
//! в docs/decisions.md). Диапазон около ±2 миллиардов, шаг сетки
//! около 0,0000000002.
//!
//! Два правила, которые важно помнить:
//! - умножение и деление округляют к нулю, поэтому `(-a) * b == -(a * b)`;
//! - переполнение и деление на ноль это паника в любой сборке. Тихо
//!   «завернувшееся» число незаметно испортило бы мир.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Сколько бит отведено под дробную часть.
pub const FRACTION_BITS: u32 = 32;

/// Единица в виде `raw`: 2^32.
const ONE_RAW: i64 = 1 << FRACTION_BITS;

/// Число с фиксированной точкой. Поле закрыто, чтобы никто снаружи
/// не перепутал `raw` с обычным целым.
///
/// Сравнение (`==`, `<`) выводится автоматически и сравнивает `raw`:
/// порядок чисел совпадает с порядком их целых представлений.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed {
    raw: i64,
}

impl Fixed {
    pub const ZERO: Fixed = Fixed { raw: 0 };
    pub const ONE: Fixed = Fixed { raw: ONE_RAW };

    /// Создаёт число из внутреннего представления. Нужно для чтения
    /// реплея и для тестов, в игровой логике не используется.
    pub fn from_raw(raw: i64) -> Fixed {
        Fixed { raw }
    }

    /// Внутреннее представление: для хеша мира и записи реплея.
    pub fn raw(self) -> i64 {
        self.raw
    }

    /// Целое число. Принимает `i32`, потому что любое `i32`, умноженное
    /// на 2^32, помещается в `i64`: переполнение здесь невозможно.
    pub fn from_int(value: i32) -> Fixed {
        Fixed {
            raw: i64::from(value) * ONE_RAW,
        }
    }

    /// Дробь `numerator / denominator`. Так записываются константы вроде
    /// 1/2 без чисел с плавающей точкой.
    pub fn from_ratio(numerator: i32, denominator: i32) -> Fixed {
        Fixed {
            raw: div_raw(
                i64::from(numerator) * ONE_RAW,
                i64::from(denominator) * ONE_RAW,
            ),
        }
    }

    /// Целая часть с округлением вниз: 2,5 → 2, но −2,5 → −3.
    /// Вниз, а не к нулю, чтобы номер клетки карты считался одинаково
    /// по обе стороны от нуля.
    pub fn floor_to_int(self) -> i64 {
        // Сдвиг вправо у знакового целого сохраняет знак и округляет вниз.
        self.raw >> FRACTION_BITS
    }

    /// Модуль числа.
    pub fn abs(self) -> Fixed {
        match self.raw.checked_abs() {
            Some(raw) => Fixed { raw },
            None => panic!("Fixed overflow in abs"),
        }
    }
}

// Сами вычисления вынесены в обычные функции над `raw`, а реализации
// операторов ниже только вызывают их. Так формула читается отдельно
// от синтаксиса трейтов.

/// Произведение. Перемножение двух `raw` даёт число с 64 дробными битами,
/// поэтому результат делится на 2^32. Промежуточное значение не помещается
/// в `i64`, считаем в `i128`.
fn mul_raw(a: i64, b: i64) -> i64 {
    let wide = i128::from(a) * i128::from(b);
    // Деление целых в Rust отбрасывает дробную часть, то есть округляет
    // к нулю. Сдвиг вправо округлял бы вниз и ломал симметрию знаков.
    narrow(wide / i128::from(ONE_RAW), "multiplication")
}

/// Частное. Делимое заранее умножается на 2^32, иначе дробная часть
/// результата потерялась бы.
fn div_raw(a: i64, b: i64) -> i64 {
    if b == 0 {
        panic!("Fixed division by zero");
    }
    let wide = i128::from(a) * i128::from(ONE_RAW);
    narrow(wide / i128::from(b), "division")
}

/// Возвращает широкий промежуточный результат в `i64`. Если он не
/// помещается, это переполнение: падаем с названием действия.
fn narrow(wide: i128, operation: &str) -> i64 {
    match i64::try_from(wide) {
        Ok(raw) => raw,
        Err(_) => panic!("Fixed overflow in {operation}"),
    }
}

impl Add for Fixed {
    type Output = Fixed;

    fn add(self, other: Fixed) -> Fixed {
        match self.raw.checked_add(other.raw) {
            Some(raw) => Fixed { raw },
            None => panic!("Fixed overflow in addition"),
        }
    }
}

impl Sub for Fixed {
    type Output = Fixed;

    fn sub(self, other: Fixed) -> Fixed {
        match self.raw.checked_sub(other.raw) {
            Some(raw) => Fixed { raw },
            None => panic!("Fixed overflow in subtraction"),
        }
    }
}

impl Mul for Fixed {
    type Output = Fixed;

    fn mul(self, other: Fixed) -> Fixed {
        Fixed {
            raw: mul_raw(self.raw, other.raw),
        }
    }
}

impl Div for Fixed {
    type Output = Fixed;

    fn div(self, other: Fixed) -> Fixed {
        Fixed {
            raw: div_raw(self.raw, other.raw),
        }
    }
}

impl Neg for Fixed {
    type Output = Fixed;

    fn neg(self) -> Fixed {
        match self.raw.checked_neg() {
            Some(raw) => Fixed { raw },
            None => panic!("Fixed overflow in negation"),
        }
    }
}

/// Печать для отладки и сообщений упавших тестов: десятичная дробь
/// с четырьмя знаками, например `-0.2500`. Цифры считаются целочисленно.
/// Знаки после четвёртого отбрасываются, поэтому два разных числа могут
/// напечататься одинаково: точное значение даёт `raw()`.
impl fmt::Debug for Fixed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // В `i128` модуль берётся без риска переполнения.
        let magnitude = i128::from(self.raw).abs();
        let one = i128::from(ONE_RAW);
        let whole = magnitude / one;
        let fraction = magnitude % one;
        let digits = fraction * 10_000 / one;
        let sign = if self.raw < 0 { "-" } else { "" };
        write!(formatter, "{sign}{whole}.{digits:04}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// 2^32 в виде `i128`: столько единиц `raw` в одной целой единице.
    const ONE_WIDE: i128 = 1 << 32;

    #[test]
    fn integer_arithmetic() {
        assert_eq!(Fixed::from_int(3) + Fixed::from_int(4), Fixed::from_int(7));
        assert_eq!(Fixed::from_int(3) - Fixed::from_int(4), Fixed::from_int(-1));
        assert_eq!(Fixed::from_int(3) * Fixed::from_int(4), Fixed::from_int(12));
        assert_eq!(Fixed::from_int(12) / Fixed::from_int(4), Fixed::from_int(3));
    }

    #[test]
    fn fraction_arithmetic() {
        assert_eq!(
            Fixed::from_ratio(1, 2) * Fixed::from_int(4),
            Fixed::from_int(2)
        );
        assert_eq!(
            Fixed::from_int(1) / Fixed::from_int(4),
            Fixed::from_ratio(1, 4)
        );
        assert_eq!(
            Fixed::from_ratio(1, 2) + Fixed::from_ratio(1, 2),
            Fixed::ONE
        );
    }

    #[test]
    fn constants() {
        assert_eq!(Fixed::ZERO, Fixed::from_int(0));
        assert_eq!(Fixed::ONE, Fixed::from_int(1));
        assert_eq!(Fixed::ONE.raw(), 1 << FRACTION_BITS);
    }

    #[test]
    fn multiplication_rounds_toward_zero() {
        // Самое маленькое число, умноженное на 1/2, не помещается в сетку.
        assert_eq!(Fixed::from_raw(1) * Fixed::from_ratio(1, 2), Fixed::ZERO);
        assert_eq!(Fixed::from_raw(-1) * Fixed::from_ratio(1, 2), Fixed::ZERO);
    }

    #[test]
    fn division_rounds_toward_zero() {
        assert_eq!(Fixed::from_raw(1) / Fixed::from_int(2), Fixed::ZERO);
        assert_eq!(Fixed::from_raw(-1) / Fixed::from_int(2), Fixed::ZERO);
    }

    #[test]
    fn floor_to_int_rounds_down() {
        assert_eq!(Fixed::from_ratio(5, 2).floor_to_int(), 2);
        assert_eq!(Fixed::from_ratio(-5, 2).floor_to_int(), -3);
        assert_eq!(Fixed::from_int(-3).floor_to_int(), -3);
    }

    #[test]
    fn abs_negation_and_ordering() {
        assert_eq!(Fixed::from_int(-3).abs(), Fixed::from_int(3));
        assert_eq!(Fixed::from_int(3).abs(), Fixed::from_int(3));
        assert_eq!(-Fixed::from_int(3), Fixed::from_int(-3));
        assert!(Fixed::from_ratio(1, 4) < Fixed::from_ratio(1, 2));
        assert!(Fixed::from_int(-1) < Fixed::ZERO);
    }

    #[test]
    #[should_panic(expected = "Fixed overflow in addition")]
    fn addition_overflow_panics() {
        let _ = Fixed::from_raw(i64::MAX) + Fixed::from_raw(1);
    }

    #[test]
    #[should_panic(expected = "Fixed overflow in multiplication")]
    fn multiplication_overflow_panics() {
        let big = Fixed::from_int(i32::MAX);
        let _ = big * big;
    }

    #[test]
    #[should_panic(expected = "Fixed division by zero")]
    fn division_by_zero_panics() {
        let _ = Fixed::ONE / Fixed::ZERO;
    }

    #[test]
    #[should_panic(expected = "Fixed division by zero")]
    fn ratio_with_zero_denominator_panics() {
        let _ = Fixed::from_ratio(1, 0);
    }

    #[test]
    fn debug_prints_decimal_fraction() {
        assert_eq!(format!("{:?}", Fixed::from_ratio(3, 2)), "1.5000");
        assert_eq!(format!("{:?}", Fixed::from_ratio(-1, 4)), "-0.2500");
        assert_eq!(format!("{:?}", Fixed::from_int(-7)), "-7.0000");
    }

    // Тесты на свойствах. `proptest` подбирает сотни случайных чисел
    // из заданных диапазонов и при провале сам ищет самый простой
    // контрпример. Диапазоны выбраны так, чтобы результат не переполнялся:
    // переполнение проверено отдельными тестами выше.
    proptest! {
        #[test]
        fn addition_is_commutative_and_reversible(
            a in -(1i64 << 62)..(1i64 << 62),
            b in -(1i64 << 62)..(1i64 << 62),
        ) {
            let a = Fixed::from_raw(a);
            let b = Fixed::from_raw(b);
            prop_assert_eq!(a + b, b + a);
            prop_assert_eq!((a + b) - b, a);
        }

        #[test]
        fn multiplication_is_commutative_and_symmetric(
            a in -(1i64 << 47)..(1i64 << 47),
            b in -(1i64 << 47)..(1i64 << 47),
        ) {
            let a = Fixed::from_raw(a);
            let b = Fixed::from_raw(b);
            prop_assert_eq!(a * b, b * a);
            prop_assert_eq!((-a) * b, -(a * b));
        }

        /// Произведение отличается от точного меньше чем на один шаг сетки
        /// и по модулю не больше точного (округление к нулю).
        #[test]
        fn multiplication_is_exact_within_one_step(
            a in -(1i64 << 47)..(1i64 << 47),
            b in -(1i64 << 47)..(1i64 << 47),
        ) {
            let product = (Fixed::from_raw(a) * Fixed::from_raw(b)).raw();
            let exact = i128::from(a) * i128::from(b);
            let scaled = i128::from(product) * ONE_WIDE;
            prop_assert!((exact - scaled).abs() < ONE_WIDE);
            prop_assert!(scaled.abs() <= exact.abs());
        }

        /// Частное, умноженное обратно на делитель, отличается от делимого
        /// меньше чем на делитель и по модулю не больше делимого.
        #[test]
        fn division_is_exact_within_one_step(
            a in -(1i64 << 30)..(1i64 << 30),
            b in (-(1i64 << 40)..(1i64 << 40)).prop_filter("делитель не ноль", |b| *b != 0),
        ) {
            let quotient = (Fixed::from_raw(a) / Fixed::from_raw(b)).raw();
            let exact = i128::from(a) * ONE_WIDE;
            let back = i128::from(quotient) * i128::from(b);
            prop_assert!((exact - back).abs() < i128::from(b).abs());
            prop_assert!(back.abs() <= exact.abs());
        }

        #[test]
        fn raw_round_trip(x in any::<i64>()) {
            prop_assert_eq!(Fixed::from_raw(x).raw(), x);
        }
    }
}
