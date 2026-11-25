//! Двумерный вектор на `Fixed`.
//!
//! Одним типом описываются и точка на плоскости карты, и смещение
//! или направление: разница только в том, как значение используют.
//! Высоты здесь нет, она появится вместе с картой в вехе 4.
//!
//! Сложение и вычитание записаны операторами. Умножение на число это
//! обычный метод `scale`, чтобы не вводить трейт с параметром типа.

use crate::fixed::Fixed;
use std::ops::{Add, Neg, Sub};

/// Вектор на плоскости. Поля открыты: это просто пара чисел без
/// скрытых правил, в отличие от `Fixed` с его внутренним `raw`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vec2Fixed {
    pub x: Fixed,
    pub y: Fixed,
}

impl Vec2Fixed {
    pub const ZERO: Vec2Fixed = Vec2Fixed {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
    };

    pub fn new(x: Fixed, y: Fixed) -> Vec2Fixed {
        Vec2Fixed { x, y }
    }

    /// Вектор с целыми координатами: удобно в тестах и сценариях.
    pub fn from_ints(x: i32, y: i32) -> Vec2Fixed {
        Vec2Fixed {
            x: Fixed::from_int(x),
            y: Fixed::from_int(y),
        }
    }

    /// Умножает обе координаты на число: растягивает или сжимает вектор.
    pub fn scale(self, factor: Fixed) -> Vec2Fixed {
        Vec2Fixed {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    /// Скалярное произведение. Для векторов длины 1 это косинус угла
    /// между ними: 1 если смотрят в одну сторону, 0 если перпендикулярны,
    /// −1 если в противоположные.
    pub fn dot(self, other: Vec2Fixed) -> Fixed {
        self.x * other.x + self.y * other.y
    }

    /// «Косое» произведение. Знак показывает, с какой стороны `other`
    /// от `self`: плюс, если поворот от `self` к `other` идёт от оси X
    /// к оси Y, минус, если наоборот, ноль, если векторы параллельны.
    pub fn perp_dot(self, other: Vec2Fixed) -> Fixed {
        self.x * other.y - self.y * other.x
    }

    /// Квадрат длины. Дешевле, чем `length`: без корня. Подходит, чтобы
    /// сравнить два расстояния. Переполняется для векторов длиннее 46 км.
    pub fn length_squared(self) -> Fixed {
        self.dot(self)
    }

    /// Длина вектора, округление вниз.
    ///
    /// Считается не через `Fixed`, а сразу в широких целых по `raw`:
    /// корень из суммы квадратов двух `raw` это и есть `raw` длины.
    /// Так квадраты не переполняются и округление происходит один раз.
    pub fn length(self) -> Fixed {
        // Модуль в беззнаковом типе существует для любого `i64`.
        let x = u128::from(self.x.raw().unsigned_abs());
        let y = u128::from(self.y.raw().unsigned_abs());

        // Каждый квадрат не больше 2^126, сумма не больше 2^127:
        // в `u128` помещается всегда.
        let sum_of_squares = x * x + y * y;
        let root = sum_of_squares.isqrt();

        // Сама длина может не поместиться, только если обе координаты
        // у самого края диапазона.
        match i64::try_from(root) {
            Ok(raw) => Fixed::from_raw(raw),
            Err(_) => panic!("Fixed overflow in length"),
        }
    }

    /// Вектор длины 1 в том же направлении. У нулевого вектора
    /// направления нет, для него возвращается `None`.
    ///
    /// Длина результата чуть меньше или чуть больше 1 из-за округления.
    /// У очень коротких векторов (длина в несколько шагов сетки)
    /// направление получается грубым.
    pub fn normalize(self) -> Option<Vec2Fixed> {
        let length = self.length();
        if length == Fixed::ZERO {
            return None;
        }
        Some(Vec2Fixed {
            x: self.x / length,
            y: self.y / length,
        })
    }
}

impl Add for Vec2Fixed {
    type Output = Vec2Fixed;

    fn add(self, other: Vec2Fixed) -> Vec2Fixed {
        Vec2Fixed {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2Fixed {
    type Output = Vec2Fixed;

    fn sub(self, other: Vec2Fixed) -> Vec2Fixed {
        Vec2Fixed {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Neg for Vec2Fixed {
    type Output = Vec2Fixed;

    fn neg(self) -> Vec2Fixed {
        Vec2Fixed {
            x: -self.x,
            y: -self.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Случайная координата в пределах ±2^46 `raw` (около ±16 тысяч):
    /// произведения двух таких чисел и их сумма не переполняют `Fixed`.
    fn small_raw() -> std::ops::Range<i64> {
        -(1i64 << 46)..(1i64 << 46)
    }

    #[test]
    fn addition_subtraction_negation_scale() {
        let a = Vec2Fixed::from_ints(1, 2);
        let b = Vec2Fixed::from_ints(3, 5);
        assert_eq!(a + b, Vec2Fixed::from_ints(4, 7));
        assert_eq!(b - a, Vec2Fixed::from_ints(2, 3));
        assert_eq!(-a, Vec2Fixed::from_ints(-1, -2));
        assert_eq!(a.scale(Fixed::from_int(3)), Vec2Fixed::from_ints(3, 6));
        assert_eq!(
            Vec2Fixed::from_ints(1, 2).scale(Fixed::from_ratio(1, 2)),
            Vec2Fixed::new(Fixed::from_ratio(1, 2), Fixed::ONE)
        );
        assert_eq!(Vec2Fixed::ZERO, Vec2Fixed::from_ints(0, 0));
    }

    #[test]
    fn dot_product() {
        let a = Vec2Fixed::from_ints(2, 3);
        let b = Vec2Fixed::from_ints(4, 5);
        assert_eq!(a.dot(b), Fixed::from_int(23));
        // Перпендикулярные направления.
        let right = Vec2Fixed::from_ints(1, 0);
        let up = Vec2Fixed::from_ints(0, 1);
        assert_eq!(right.dot(up), Fixed::ZERO);
    }

    #[test]
    fn perp_dot_product() {
        let right = Vec2Fixed::from_ints(1, 0);
        let up = Vec2Fixed::from_ints(0, 1);
        assert_eq!(right.perp_dot(up), Fixed::ONE);
        assert_eq!(up.perp_dot(right), Fixed::from_int(-1));
        // Параллельные векторы.
        assert_eq!(right.perp_dot(Vec2Fixed::from_ints(7, 0)), Fixed::ZERO);
    }

    #[test]
    fn length_and_length_squared() {
        assert_eq!(Vec2Fixed::from_ints(3, 4).length(), Fixed::from_int(5));
        assert_eq!(Vec2Fixed::from_ints(-3, -4).length(), Fixed::from_int(5));
        assert_eq!(Vec2Fixed::ZERO.length(), Fixed::ZERO);
        assert_eq!(
            Vec2Fixed::from_ints(3, 4).length_squared(),
            Fixed::from_int(25)
        );
    }

    #[test]
    fn length_works_where_squares_do_not_fit() {
        // 3 000 000 в квадрате это 9 * 10^12, в `Fixed` не помещается,
        // но длина считается в широких целых.
        let far = Vec2Fixed::from_ints(3_000_000, 4_000_000);
        assert_eq!(far.length(), Fixed::from_int(5_000_000));
    }

    #[test]
    #[should_panic(expected = "Fixed overflow in length")]
    fn length_overflow_panics() {
        let corner = Vec2Fixed::new(Fixed::from_raw(i64::MAX), Fixed::from_raw(i64::MAX));
        let _ = corner.length();
    }

    #[test]
    fn normalize_gives_unit_direction() {
        assert_eq!(
            Vec2Fixed::from_ints(5, 0).normalize(),
            Some(Vec2Fixed::from_ints(1, 0))
        );
        assert_eq!(
            Vec2Fixed::from_ints(3, 4).normalize(),
            Some(Vec2Fixed::new(
                Fixed::from_ratio(3, 5),
                Fixed::from_ratio(4, 5)
            ))
        );
    }

    #[test]
    fn normalize_of_zero_is_none() {
        assert_eq!(Vec2Fixed::ZERO.normalize(), None);
    }

    proptest! {
        /// Длина это наибольшее число сетки, квадрат которого не больше
        /// суммы квадратов координат. Проверяется в `u128` по `raw`.
        #[test]
        fn length_is_the_largest_that_fits(
            x in -(1i64 << 62)..(1i64 << 62),
            y in -(1i64 << 62)..(1i64 << 62),
        ) {
            let length = Vec2Fixed::new(Fixed::from_raw(x), Fixed::from_raw(y)).length().raw();
            let length = u128::try_from(length).expect("длина не отрицательна");
            let x = u128::from(x.unsigned_abs());
            let y = u128::from(y.unsigned_abs());
            let sum = x * x + y * y;
            prop_assert!(length * length <= sum);
            prop_assert!((length + 1) * (length + 1) > sum);
        }

        /// У не слишком короткого вектора нормализованная длина почти 1,
        /// а направление то же (скалярное произведение положительно).
        #[test]
        fn normalized_length_is_close_to_one(
            x in -(1i64 << 52)..(1i64 << 52),
            y in -(1i64 << 52)..(1i64 << 52),
        ) {
            let vector = Vec2Fixed::new(Fixed::from_raw(x), Fixed::from_raw(y));
            prop_assume!(vector.length() >= Fixed::from_ratio(1, 1024));
            let unit = vector.normalize().expect("вектор не нулевой");
            let error = (unit.length() - Fixed::ONE).abs();
            // 2^12 единиц `raw` это 2^-20.
            prop_assert!(error < Fixed::from_raw(1 << 12));
            prop_assert!(vector.dot(unit) > Fixed::ZERO);
        }

        #[test]
        fn dot_is_commutative_and_perp_dot_is_antisymmetric(
            ax in small_raw(), ay in small_raw(), bx in small_raw(), by in small_raw(),
        ) {
            let a = Vec2Fixed::new(Fixed::from_raw(ax), Fixed::from_raw(ay));
            let b = Vec2Fixed::new(Fixed::from_raw(bx), Fixed::from_raw(by));
            prop_assert_eq!(a.dot(b), b.dot(a));
            prop_assert_eq!(a.perp_dot(b), -b.perp_dot(a));
        }
    }
}
