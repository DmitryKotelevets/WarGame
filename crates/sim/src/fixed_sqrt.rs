//! Квадратный корень для `Fixed`.
//!
//! Корень считается целочисленно, стандартным методом `isqrt`: он возвращает
//! наибольшее целое, квадрат которого не больше аргумента. Чисел с плавающей
//! точкой внутри нет, поэтому результат одинаков на любом процессоре.
//!
//! Метод лежит отдельно от `fixed.rs`, чтобы тот файл не перерос лимит
//! в 400 строк. Поле `raw` отсюда не видно, поэтому используются
//! `raw()` и `from_raw()`.

use crate::fixed::{FRACTION_BITS, Fixed};

impl Fixed {
    /// Квадратный корень с округлением вниз. Отрицательный аргумент
    /// это ошибка в вызывающем коде, поэтому паника.
    pub fn sqrt(self) -> Fixed {
        // Перевод в беззнаковое целое заодно ловит отрицательное число.
        let raw = match u128::try_from(self.raw()) {
            Ok(raw) => raw,
            Err(_) => panic!("Fixed square root of a negative number"),
        };

        // Значение равно raw / 2^32. Корень из него это
        // sqrt(raw) / 2^16, и половина дробных битов потерялась бы.
        // Поэтому сначала умножаем на 2^32: sqrt(raw * 2^32) / 2^32.
        let scaled = raw * (1u128 << FRACTION_BITS);
        let root = scaled.isqrt();

        // Корень из числа меньше 2^95 меньше 2^48 и всегда помещается.
        match i64::try_from(root) {
            Ok(raw) => Fixed::from_raw(raw),
            Err(_) => panic!("Fixed overflow in square root"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fixed::Fixed;
    use proptest::prelude::*;

    #[test]
    fn exact_roots() {
        assert_eq!(Fixed::from_int(4).sqrt(), Fixed::from_int(2));
        assert_eq!(Fixed::from_ratio(1, 4).sqrt(), Fixed::from_ratio(1, 2));
        assert_eq!(Fixed::ZERO.sqrt(), Fixed::ZERO);
        assert_eq!(Fixed::ONE.sqrt(), Fixed::ONE);
    }

    #[test]
    #[should_panic(expected = "Fixed square root of a negative number")]
    fn negative_argument_panics() {
        let _ = Fixed::from_int(-1).sqrt();
    }

    proptest! {
        /// Корень это наибольшее число сетки, квадрат которого не больше
        /// аргумента: `r² <= x < (r + шаг)²`. Проверяется в `u128` по `raw`,
        /// где `x` с 64 дробными битами это `x_raw * 2^32`.
        #[test]
        fn root_is_the_largest_that_fits(x in 0i64..=i64::MAX) {
            let root = Fixed::from_raw(x).sqrt().raw();
            let root = u128::try_from(root).expect("корень не отрицателен");
            let x_wide = u128::try_from(x).expect("аргумент не отрицателен") * (1u128 << 32);
            prop_assert!(root * root <= x_wide);
            prop_assert!((root + 1) * (root + 1) > x_wide);
        }
    }
}
