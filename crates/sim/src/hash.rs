//! Хеш состояния мира.
//!
//! Хеш сворачивает всё состояние мира в одно число. Сравнить два числа
//! гораздо проще, чем два мира: если хеши двух прогонов после одного
//! и того же тика равны, прогоны идут одинаково, если нет, они разошлись.
//! На этом стоят тест детерминизма, проверка реплея и, в будущем,
//! поиск расхождений в сетевой игре.
//!
//! Алгоритм называется FNV-1a на 64 бита (общественное достояние).
//! Он обрабатывает вход по одному байту: смешивает байт с состоянием
//! и умножает состояние на константу. Хеш не защищает от намеренной
//! подделки, но случайно совпасть у двух разных миров может с вероятностью
//! порядка 2^-64, а это всё, что от него нужно.
//!
//! `std::hash` из стандартной библиотеки не используется: его результат
//! не обещан одинаковым между версиями Rust и платформами.
//!
//! Что именно и в каком порядке попадает в хеш, записано в `World::hash`,
//! `UnitStore::hash_into` и `Unit::hash_into`. Порядок это часть формата:
//! поменять его значит поменять все хеши.

use std::fmt;

/// Начальное состояние и множитель из описания FNV-1a.
const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// Хеш мира. Отдельный тип, чтобы его нельзя было спутать с сидом
/// или другим `u64`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WorldHash(u64);

impl WorldHash {
    /// Хеш из числа: для чтения эталона или реплея.
    pub fn from_raw(raw: u64) -> WorldHash {
        WorldHash(raw)
    }

    /// Хеш обычным числом.
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Печать для человека: всегда 16 шестнадцатеричных цифр, с ведущими
/// нулями. Так хеши удобно сравнивать глазами в логе.
impl fmt::Display for WorldHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Отладочная печать (`{:?}`, сообщения `assert_eq!`) в том же виде.
impl fmt::Debug for WorldHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WorldHash({self})")
    }
}

/// Накопитель хеша: в него по очереди записывают поля мира, в конце
/// забирают результат.
pub struct StateHasher {
    state: u64,
}

impl StateHasher {
    /// Накопитель, в который ещё ничего не записано.
    pub fn new() -> StateHasher {
        StateHasher {
            state: OFFSET_BASIS,
        }
    }

    /// Записывает один байт. Это и есть весь алгоритм FNV-1a; остальные
    /// методы только режут числа на байты.
    pub fn write_u8(&mut self, byte: u8) {
        self.state ^= u64::from(byte);
        // Заворачивание при переполнении это часть алгоритма, как в `Rng`.
        self.state = self.state.wrapping_mul(PRIME);
    }

    /// Записывает `false` как байт 0, `true` как байт 1.
    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    /// Записывает число по байтам, начиная с младшего. Порядок выбран
    /// явно (`to_le_bytes`), поэтому не зависит от процессора.
    pub fn write_u32(&mut self, value: u32) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    pub fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    pub fn write_i64(&mut self, value: i64) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    /// Готовый хеш всего записанного.
    pub fn finish(self) -> WorldHash {
        WorldHash(self.state)
    }
}

/// Clippy просит `Default` у типа с `new()` без аргументов. Написан
/// вручную: выведенный автоматически начал бы с нуля, а не с константы
/// алгоритма.
impl Default for StateHasher {
    fn default() -> StateHasher {
        StateHasher::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Хеш последовательности байтов.
    fn hash_of_bytes(bytes: &[u8]) -> u64 {
        let mut hasher = StateHasher::new();
        for byte in bytes {
            hasher.write_u8(*byte);
        }
        hasher.finish().raw()
    }

    #[test]
    fn matches_fnv1a_reference_values() {
        // Опубликованные эталоны FNV-1a на 64 бита.
        assert_eq!(hash_of_bytes(b""), 0xcbf29ce484222325);
        assert_eq!(hash_of_bytes(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(hash_of_bytes(b"foobar"), 0x85944171f73967e8);
    }

    #[test]
    fn wide_numbers_are_written_low_byte_first() {
        let mut hasher = StateHasher::new();
        hasher.write_u32(0x0403_0201);
        assert_eq!(hasher.finish().raw(), hash_of_bytes(&[1, 2, 3, 4]));

        let mut hasher = StateHasher::new();
        hasher.write_u64(0x0807_0605_0403_0201);
        assert_eq!(
            hasher.finish().raw(),
            hash_of_bytes(&[1, 2, 3, 4, 5, 6, 7, 8])
        );

        let mut hasher = StateHasher::new();
        hasher.write_i64(0x0807_0605_0403_0201);
        assert_eq!(
            hasher.finish().raw(),
            hash_of_bytes(&[1, 2, 3, 4, 5, 6, 7, 8])
        );

        // Минус один в дополнительном коде это все единицы.
        let mut hasher = StateHasher::new();
        hasher.write_i64(-1);
        assert_eq!(hasher.finish().raw(), hash_of_bytes(&[0xff; 8]));
    }

    #[test]
    fn bool_is_written_as_one_byte() {
        let mut hasher = StateHasher::new();
        hasher.write_bool(false);
        hasher.write_bool(true);
        assert_eq!(hasher.finish().raw(), hash_of_bytes(&[0, 1]));
    }

    #[test]
    fn order_of_writes_matters() {
        assert_ne!(hash_of_bytes(&[1, 2]), hash_of_bytes(&[2, 1]));
    }

    #[test]
    fn hash_prints_as_sixteen_hex_digits() {
        let mut hasher = StateHasher::new();
        hasher.write_u8(b'a');
        let hash = hasher.finish();
        assert_eq!(format!("{hash}"), "af63dc4c8601ec8c");
        assert_eq!(format!("{hash:?}"), "WorldHash(af63dc4c8601ec8c)");
        // Ведущие нули не теряются.
        assert_eq!(format!("{}", WorldHash::from_raw(0x1f)), "000000000000001f");
    }
}
