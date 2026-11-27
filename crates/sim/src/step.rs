//! Один тик симуляции.
//!
//! `World::step()` это единственное место, где мир меняется во времени,
//! и единственное место, где записан порядок работы внутри тика.
//! Порядок это часть правил игры: если поменять местами две системы,
//! тот же реплей даст другой результат. Поэтому системы вызываются
//! обычными строками подряд, сверху вниз, без списков и регистрации:
//! порядок виден глазами и проходится отладчиком.
//!
//! Новая система добавляется сюда одной строкой, а её код живёт в своём
//! модуле.

use crate::world::World;

impl World {
    /// Продвигает мир ровно на один тик.
    pub fn step(&mut self) {
        // Здесь появятся (в этом порядке):
        // - применение команд этого тика (шаг 2.7);
        // - движение юнитов (шаг 2.8).

        // Удаление идёт после всех систем: каждая из них видела один
        // и тот же набор юнитов, а снаружи (клиент, хеш) после шага
        // отмеченных уже нет.
        self.units.remove_marked();

        // Счётчик растёт последним: всё, что выше, работало с номером
        // тика, который сейчас считается. После шага `tick` равен числу
        // выполненных шагов.
        self.tick = self.tick.next();
    }
}

#[cfg(test)]
mod tests {
    use crate::tick::Tick;
    use crate::unit::UnitId;
    use crate::vec2::Vec2Fixed;
    use crate::world::World;

    #[test]
    fn step_advances_tick_by_one() {
        let mut world = World::new(0);
        world.step();
        assert_eq!(world.tick, Tick::from_raw(1));
    }

    #[test]
    fn hundred_steps_give_tick_hundred() {
        let mut world = World::new(0);
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(world.tick, Tick::from_raw(100));
    }

    #[test]
    fn step_in_empty_world_keeps_it_empty() {
        let mut world = World::new(0);
        world.step();
        assert!(world.units.is_empty());
    }

    #[test]
    fn step_does_not_touch_the_rng() {
        // Пока ни одна система не берёт случайных чисел.
        let mut world = World::new(123);
        let state_before = world.rng.state();
        world.step();
        assert_eq!(world.rng.state(), state_before);
    }

    #[test]
    fn step_removes_marked_and_keeps_the_rest() {
        let mut world = World::new(0);
        let stays = world.units.spawn(Vec2Fixed::from_ints(1, 2));
        let dies = world.units.spawn(Vec2Fixed::from_ints(3, 4));
        world.units.mark_for_removal(dies);

        world.step();

        assert_eq!(world.units.get(dies), None);
        assert_eq!(world.units.len(), 1);
        let unit = world.units.get(stays).expect("юнит никто не отмечал");
        assert_eq!(unit.id, stays);
        assert_eq!(unit.position, Vec2Fixed::from_ints(1, 2));
    }

    #[test]
    fn marked_unit_is_removed_by_the_same_step() {
        let mut world = World::new(0);
        let dies = world.units.spawn(Vec2Fixed::ZERO);
        world.units.mark_for_removal(dies);

        world.step();

        // Ячейка уже свободна: новый юнит занимает её со следующим
        // поколением. Если бы удаление отложилось на следующий шаг,
        // новый юнит получил бы ячейку 1.
        let new = world.units.spawn(Vec2Fixed::ZERO);
        assert_eq!(
            new,
            UnitId {
                index: 0,
                generation: 1
            }
        );
    }
}
