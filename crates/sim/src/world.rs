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

use crate::hash::{StateHasher, WorldHash};
use crate::rng::Rng;
use crate::tick::Tick;
use crate::unit_store::UnitStore;

/// Всё состояние симуляции.
pub struct World {
    /// Сколько шагов уже выполнено. Во время шага это номер тика,
    /// который сейчас считается.
    pub tick: Tick,
    /// Все юниты.
    pub units: UnitStore,
    /// Единственный источник случайности в игре.
    pub rng: Rng,
}

impl World {
    /// Мир в начале игры: тик 0, юнитов нет. Сид задаёт всю будущую
    /// случайность: два мира с одним сидом и одними командами проживут
    /// одинаковую жизнь.
    pub fn new(seed: u64) -> World {
        World {
            tick: Tick::ZERO,
            units: UnitStore::new(),
            rng: Rng::new(seed),
        }
    }

    /// Хеш всего состояния мира. Мир не меняет.
    ///
    /// Порядок записи: тик, генератор, юниты. Порядок это часть формата
    /// хеша: менять его можно только вместе с эталоном в тесте
    /// `reference_hash`.
    pub fn hash(&self) -> WorldHash {
        // Разбор на поля целиком: новое поле мира без строки в хеше
        // не скомпилируется.
        let World { tick, units, rng } = self;

        let mut hasher = StateHasher::new();
        hasher.write_u32(tick.raw());
        hasher.write_u64(rng.state());
        units.hash_into(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Явное имя: в `proptest::prelude` есть свой `Rng` (см. `rng.rs`).
    use crate::rng::Rng;
    use crate::vec2::Vec2Fixed;
    use proptest::prelude::*;

    #[test]
    fn new_world_is_empty_at_tick_zero() {
        let world = World::new(0);
        assert_eq!(world.tick, Tick::ZERO);
        assert!(world.units.is_empty());
    }

    #[test]
    fn new_world_seeds_its_rng() {
        let world = World::new(777);
        assert_eq!(world.rng, Rng::new(777));
        // Другой сид это другой мир.
        assert_ne!(World::new(778).rng, world.rng);
    }

    /// Небольшой мир для тестов хеша: сид 5, два юнита, три шага.
    fn sample_world() -> World {
        let mut world = World::new(5);
        world.units.spawn(Vec2Fixed::from_ints(1, 2));
        world.units.spawn(Vec2Fixed::from_ints(-3, 4));
        for _ in 0..3 {
            world.step();
        }
        world
    }

    #[test]
    fn equal_worlds_have_equal_hashes() {
        let first = sample_world();
        let second = sample_world();
        assert_eq!(first.hash(), second.hash());
        // Подсчёт хеша мир не меняет.
        assert_eq!(first.hash(), first.hash());
    }

    #[test]
    fn hash_depends_on_seed() {
        assert_ne!(World::new(1).hash(), World::new(2).hash());
    }

    #[test]
    fn hash_depends_on_tick() {
        let mut world = sample_world();
        let before = world.hash();
        world.step();
        assert_ne!(world.hash(), before);
    }

    #[test]
    fn hash_depends_on_rng_state() {
        let mut world = sample_world();
        let before = world.hash();
        world.rng.next_u64();
        assert_ne!(world.hash(), before);
    }

    #[test]
    fn hash_depends_on_unit_count() {
        let mut world = sample_world();
        let before = world.hash();
        world.units.spawn(Vec2Fixed::ZERO);
        assert_ne!(world.hash(), before);
    }

    #[test]
    fn hash_depends_on_each_coordinate() {
        let mut first = World::new(0);
        first.units.spawn(Vec2Fixed::from_ints(1, 2));
        let mut second = World::new(0);
        second.units.spawn(Vec2Fixed::from_ints(2, 1));
        assert_ne!(first.hash(), second.hash());

        // Сдвиг на самый маленький шаг сетки тоже виден.
        let mut third = World::new(0);
        let id = third.units.spawn(Vec2Fixed::from_ints(1, 2));
        let unit = third.units.get_mut(id).expect("юнит только что создан");
        unit.position.x = crate::fixed::Fixed::from_raw(unit.position.x.raw() + 1);
        assert_ne!(first.hash(), third.hash());
    }

    #[test]
    fn hash_depends_on_removal_mark() {
        let mut world = World::new(0);
        let id = world.units.spawn(Vec2Fixed::ZERO);
        let before = world.hash();
        world.units.mark_for_removal(id);
        assert_ne!(world.hash(), before);
    }

    #[test]
    fn hash_remembers_a_unit_that_lived_and_died() {
        // В обоих мирах сейчас нет юнитов и тик равен 1, но в первом
        // ячейка 0 уже была занята: следующий юнит получит в нём
        // поколение 1, а во втором поколение 0. Это разные состояния.
        let mut first = World::new(0);
        let id = first.units.spawn(Vec2Fixed::ZERO);
        first.units.mark_for_removal(id);
        first.step();

        let mut second = World::new(0);
        second.step();

        assert!(first.units.is_empty());
        assert!(second.units.is_empty());
        assert_ne!(first.hash(), second.hash());
    }

    #[test]
    fn reference_hash() {
        // Эталон. Этот тест падает при любом изменении формата хеша или
        // логики симуляции, после которого старые реплеи перестают
        // сходиться. Число меняется только с согласия автора, обычно
        // вместе с `SIM_VERSION`.
        let mut world = World::new(2026);
        world.units.spawn(Vec2Fixed::from_ints(3, 4));
        let removed = world.units.spawn(Vec2Fixed::from_ints(-5, 6));
        let marked = world.units.spawn(Vec2Fixed::from_ints(7, -8));
        world.units.mark_for_removal(removed);
        for _ in 0..10 {
            world.step();
        }
        world.units.mark_for_removal(marked);

        assert_eq!(world.hash(), WorldHash::from_raw(0x0676_4738_3605_609a));
    }

    /// Выполняет одно действие над миром: создать юнит, отметить живого
    /// или сделать шаг.
    fn apply(world: &mut World, kind: u8, pick: usize, x: i32, y: i32) {
        if kind == 0 {
            world.units.spawn(Vec2Fixed::from_ints(x, y));
            return;
        }
        if kind == 1 {
            let mut ids = Vec::new();
            for unit in world.units.iter() {
                ids.push(unit.id);
            }
            if !ids.is_empty() {
                world.units.mark_for_removal(ids[pick % ids.len()]);
            }
            return;
        }
        world.step();
    }

    proptest! {
        /// Одни и те же действия над двумя мирами дают равные хеши
        /// после каждого действия.
        #[test]
        fn same_actions_give_same_hashes(
            seed in any::<u64>(),
            actions in prop::collection::vec(
                (0u8..3, any::<usize>(), -1000i32..1000, -1000i32..1000),
                0..100,
            ),
        ) {
            let mut first = World::new(seed);
            let mut second = World::new(seed);
            prop_assert_eq!(first.hash(), second.hash());
            for (kind, pick, x, y) in actions {
                apply(&mut first, kind, pick, x, y);
                apply(&mut second, kind, pick, x, y);
                prop_assert_eq!(first.hash(), second.hash());
            }
        }
    }
}
