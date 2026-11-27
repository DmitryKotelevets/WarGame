//! Хранилище юнитов.
//!
//! Юниты лежат в одном `Vec` ячеек. Ячейка либо занята юнитом, либо
//! свободна и ждёт следующего. Номер ячейки это `index` в `UnitId`,
//! а счётчик `generation` в ячейке отличает нынешнего жильца от прежних
//! (подробно в `unit.rs`).
//!
//! Три решения, которые важны для детерминизма:
//! - Обход идёт по возрастанию номера ячейки, всегда одинаково.
//! - Новый юнит занимает свободную ячейку с наименьшим номером. Она
//!   ищется простым проходом. Отдельного списка свободных ячеек нет:
//!   его порядок был бы скрытым состоянием, от которого зависят будущие
//!   идентификаторы, и его пришлось бы хешировать и сохранять.
//! - Юнит не удаляется посреди тика. Системы только ставят отметку,
//!   а `remove_marked` в конце тика освобождает ячейки. Так все системы
//!   одного тика видят один и тот же набор юнитов.

use crate::hash::StateHasher;
use crate::unit::{Unit, UnitId};
use crate::vec2::Vec2Fixed;

/// Одна ячейка хранилища.
struct Slot {
    /// Сколько раз ячейку освобождали. Идентификатор подходит к ячейке,
    /// только если его поколение равно этому числу.
    generation: u32,
    /// `Some(юнит)`, если ячейка занята, `None`, если свободна.
    unit: Option<Unit>,
    /// Юнит в ячейке будет удалён в конце тика.
    marked_for_removal: bool,
}

/// Все юниты мира.
#[derive(Default)]
pub struct UnitStore {
    /// Закрыто: менять ячейки в обход методов нельзя, иначе разойдутся
    /// поколения и идентификаторы.
    slots: Vec<Slot>,
}

/// Номер ячейки из идентификатора в виде, пригодном для индексации `Vec`.
fn slot_index(id: UnitId) -> usize {
    match usize::try_from(id.index) {
        Ok(index) => index,
        // Возможно только на 16-битной платформе, игра на таких не идёт.
        Err(_) => panic!("UnitStore: unit index does not fit in usize"),
    }
}

impl UnitStore {
    /// Пустое хранилище.
    pub fn new() -> UnitStore {
        UnitStore { slots: Vec::new() }
    }

    /// Создаёт юнит в точке `position` и возвращает его идентификатор.
    pub fn spawn(&mut self, position: Vec2Fixed) -> UnitId {
        // Ищем свободную ячейку с наименьшим номером.
        let mut free_index = None;
        for (index, slot) in self.slots.iter().enumerate() {
            if slot.unit.is_none() {
                free_index = Some(index);
                break;
            }
        }
        // Свободных нет: добавляем новую ячейку в конец.
        let index = match free_index {
            Some(index) => index,
            None => {
                self.slots.push(Slot {
                    generation: 0,
                    unit: None,
                    marked_for_removal: false,
                });
                self.slots.len() - 1
            }
        };

        let id = UnitId {
            index: match u32::try_from(index) {
                Ok(index) => index,
                Err(_) => panic!("UnitStore: too many unit slots"),
            },
            generation: self.slots[index].generation,
        };
        self.slots[index].unit = Some(Unit { id, position });
        id
    }

    /// Юнит по идентификатору. `None`, если такого юнита нет: он уже
    /// удалён или идентификатор никогда не выдавался. Это обычная
    /// ситуация (цель погибла), вызывающий код обязан её разобрать.
    pub fn get(&self, id: UnitId) -> Option<&Unit> {
        let slot = self.slots.get(slot_index(id))?;
        if slot.generation != id.generation {
            return None;
        }
        slot.unit.as_ref()
    }

    /// То же, что `get`, но юнит можно менять.
    pub fn get_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        let slot = self.slots.get_mut(slot_index(id))?;
        if slot.generation != id.generation {
            return None;
        }
        slot.unit.as_mut()
    }

    /// Отмечает юнит на удаление в конце тика. До `remove_marked` юнит
    /// остаётся на месте и виден всем. Повторная отметка ничего не меняет:
    /// двое могут убить одного за один тик.
    ///
    /// Если такого юнита нет, это паника: удалять можно только того,
    /// кого вызывающий код только что нашёл.
    pub fn mark_for_removal(&mut self, id: UnitId) {
        let slot = match self.slots.get_mut(slot_index(id)) {
            Some(slot) => slot,
            None => panic!("UnitStore::mark_for_removal: no such unit {id:?}"),
        };
        if slot.generation != id.generation || slot.unit.is_none() {
            panic!("UnitStore::mark_for_removal: no such unit {id:?}");
        }
        slot.marked_for_removal = true;
    }

    /// Удаляет всех отмеченных. Вызывается один раз в конце тика.
    /// Ячейка освобождается, её поколение растёт на 1, и с этого момента
    /// старый идентификатор никого не находит.
    pub fn remove_marked(&mut self) {
        for slot in &mut self.slots {
            if !slot.marked_for_removal {
                continue;
            }
            slot.unit = None;
            slot.marked_for_removal = false;
            slot.generation = match slot.generation.checked_add(1) {
                Some(generation) => generation,
                // Четыре миллиарда смертей в одной ячейке. Завернуться
                // в ноль нельзя: ожили бы самые старые идентификаторы.
                None => panic!("UnitStore: generation overflow"),
            };
        }
    }

    /// Все юниты по возрастанию номера ячейки, включая отмеченных
    /// на удаление.
    pub fn iter(&self) -> impl Iterator<Item = &Unit> {
        self.slots.iter().filter_map(|slot| slot.unit.as_ref())
    }

    /// То же, что `iter`, но юниты можно менять.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Unit> {
        self.slots.iter_mut().filter_map(|slot| slot.unit.as_mut())
    }

    /// Число юнитов, включая отмеченных на удаление.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Юнитов нет совсем.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Записывает хранилище в хеш мира. Функция живёт здесь, потому что
    /// ячейки закрыты.
    ///
    /// Свободные ячейки тоже записываются: их поколения определяют,
    /// какие идентификаторы получат будущие юниты.
    pub fn hash_into(&self, hasher: &mut StateHasher) {
        // Разбор на поля целиком: новое поле без строки в хеше
        // не скомпилируется (то же в `Unit::hash_into`).
        let UnitStore { slots } = self;

        let slot_count = match u64::try_from(slots.len()) {
            Ok(count) => count,
            Err(_) => panic!("UnitStore: slot count does not fit in u64"),
        };
        hasher.write_u64(slot_count);

        for slot in slots {
            let Slot {
                generation,
                unit,
                marked_for_removal,
            } = slot;

            hasher.write_u32(*generation);
            hasher.write_bool(*marked_for_removal);
            match unit {
                Some(unit) => {
                    hasher.write_bool(true);
                    unit.hash_into(hasher);
                }
                None => hasher.write_bool(false),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn id(index: u32, generation: u32) -> UnitId {
        UnitId { index, generation }
    }

    /// Идентификаторы всех юнитов в порядке обхода.
    fn ids_in_order(store: &UnitStore) -> Vec<UnitId> {
        let mut ids = Vec::new();
        for unit in store.iter() {
            ids.push(unit.id);
        }
        ids
    }

    #[test]
    fn spawn_then_get() {
        let mut store = UnitStore::new();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());

        let first = store.spawn(Vec2Fixed::from_ints(10, 20));
        let second = store.spawn(Vec2Fixed::from_ints(30, 40));
        assert_eq!(first, id(0, 0));
        assert_eq!(second, id(1, 0));
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());

        let unit = store.get(first).expect("юнит только что создан");
        assert_eq!(unit.id, first);
        assert_eq!(unit.position, Vec2Fixed::from_ints(10, 20));
        let unit = store.get(second).expect("юнит только что создан");
        assert_eq!(unit.id, second);
        assert_eq!(unit.position, Vec2Fixed::from_ints(30, 40));
    }

    #[test]
    fn get_with_unknown_id_is_none() {
        let mut store = UnitStore::new();
        // Пустое хранилище.
        assert_eq!(store.get(id(0, 0)), None);

        store.spawn(Vec2Fixed::ZERO);
        // Индекс за пределами.
        assert_eq!(store.get(id(1, 0)), None);
        assert_eq!(store.get(id(u32::MAX, 0)), None);
        // Чужое поколение.
        assert_eq!(store.get(id(0, 1)), None);
        assert_eq!(store.get_mut(id(0, 1)), None);
    }

    #[test]
    fn get_mut_changes_the_stored_unit() {
        let mut store = UnitStore::new();
        let unit_id = store.spawn(Vec2Fixed::ZERO);
        let unit = store.get_mut(unit_id).expect("юнит только что создан");
        unit.position = Vec2Fixed::from_ints(7, 8);
        let unit = store.get(unit_id).expect("юнит никто не удалял");
        assert_eq!(unit.position, Vec2Fixed::from_ints(7, 8));
    }

    #[test]
    fn marked_unit_lives_until_remove_marked() {
        let mut store = UnitStore::new();
        let stays = store.spawn(Vec2Fixed::from_ints(1, 1));
        let dies = store.spawn(Vec2Fixed::from_ints(2, 2));

        store.mark_for_removal(dies);
        // До конца тика юнит виден всем системам.
        assert!(store.get(dies).is_some());
        assert_eq!(store.len(), 2);
        assert_eq!(ids_in_order(&store), vec![stays, dies]);

        store.remove_marked();
        assert_eq!(store.get(dies), None);
        assert_eq!(store.get_mut(dies), None);
        assert!(store.get(stays).is_some());
        assert_eq!(store.len(), 1);
        assert_eq!(ids_in_order(&store), vec![stays]);
    }

    #[test]
    fn remove_marked_without_marks_changes_nothing() {
        let mut store = UnitStore::new();
        let unit_id = store.spawn(Vec2Fixed::ZERO);
        store.remove_marked();
        assert!(store.get(unit_id).is_some());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn reused_slot_gets_next_generation() {
        let mut store = UnitStore::new();
        let old = store.spawn(Vec2Fixed::from_ints(1, 1));
        store.mark_for_removal(old);
        store.remove_marked();

        let new = store.spawn(Vec2Fixed::from_ints(2, 2));
        assert_eq!(new, id(0, 1));
        // Старый идентификатор не находит нового жильца той же ячейки.
        assert_eq!(store.get(old), None);
        let unit = store.get(new).expect("юнит только что создан");
        assert_eq!(unit.id, new);
        assert_eq!(unit.position, Vec2Fixed::from_ints(2, 2));
    }

    #[test]
    fn spawn_takes_the_lowest_free_slot() {
        let mut store = UnitStore::new();
        let a = store.spawn(Vec2Fixed::ZERO);
        let b = store.spawn(Vec2Fixed::ZERO);
        let c = store.spawn(Vec2Fixed::ZERO);
        // Порядок отметок не должен влиять на порядок выдачи ячеек.
        store.mark_for_removal(c);
        store.mark_for_removal(a);
        store.remove_marked();

        assert_eq!(store.spawn(Vec2Fixed::ZERO), id(0, 1));
        assert_eq!(store.spawn(Vec2Fixed::ZERO), id(2, 1));
        assert_eq!(store.spawn(Vec2Fixed::ZERO), id(3, 0));
        assert!(store.get(b).is_some());
    }

    #[test]
    fn iteration_goes_by_ascending_index() {
        let mut store = UnitStore::new();
        let a = store.spawn(Vec2Fixed::ZERO);
        let b = store.spawn(Vec2Fixed::ZERO);
        let c = store.spawn(Vec2Fixed::ZERO);
        let d = store.spawn(Vec2Fixed::ZERO);
        store.mark_for_removal(b);
        store.mark_for_removal(a);
        store.remove_marked();
        assert_eq!(ids_in_order(&store), vec![c, d]);

        // Новый юнит занял ячейку 0 и в обходе идёт первым, хотя создан
        // последним.
        let e = store.spawn(Vec2Fixed::ZERO);
        assert_eq!(ids_in_order(&store), vec![e, c, d]);
    }

    #[test]
    fn iter_mut_changes_every_unit() {
        let mut store = UnitStore::new();
        let a = store.spawn(Vec2Fixed::from_ints(1, 0));
        let b = store.spawn(Vec2Fixed::from_ints(2, 0));
        for unit in store.iter_mut() {
            unit.position = unit.position + Vec2Fixed::from_ints(0, 5);
        }
        let unit = store.get(a).expect("юнит никто не удалял");
        assert_eq!(unit.position, Vec2Fixed::from_ints(1, 5));
        let unit = store.get(b).expect("юнит никто не удалял");
        assert_eq!(unit.position, Vec2Fixed::from_ints(2, 5));
    }

    #[test]
    fn marking_twice_removes_once() {
        let mut store = UnitStore::new();
        let unit_id = store.spawn(Vec2Fixed::ZERO);
        store.mark_for_removal(unit_id);
        store.mark_for_removal(unit_id);
        store.remove_marked();
        assert_eq!(store.len(), 0);
        // Поколение выросло на 1, а не на 2.
        assert_eq!(store.spawn(Vec2Fixed::ZERO), id(0, 1));
    }

    #[test]
    #[should_panic(expected = "mark_for_removal: no such unit")]
    fn marking_a_removed_unit_panics() {
        let mut store = UnitStore::new();
        let unit_id = store.spawn(Vec2Fixed::ZERO);
        store.mark_for_removal(unit_id);
        store.remove_marked();
        store.mark_for_removal(unit_id);
    }

    #[test]
    #[should_panic(expected = "mark_for_removal: no such unit")]
    fn marking_an_out_of_range_id_panics() {
        let mut store = UnitStore::new();
        store.mark_for_removal(id(5, 0));
    }

    #[test]
    #[should_panic(expected = "generation overflow")]
    fn generation_overflow_panics() {
        let mut store = UnitStore::new();
        store.spawn(Vec2Fixed::ZERO);
        // Четыре миллиарда смертей честно не прокрутить, поэтому тест
        // выставляет счётчик ячейки напрямую.
        store.slots[0].generation = u32::MAX;
        store.slots[0].marked_for_removal = true;
        store.remove_marked();
    }

    /// Простая модель хранилища для сверки: кто сейчас жив, кто отмечен
    /// на удаление и кто уже удалён.
    struct Model {
        alive: Vec<(UnitId, Vec2Fixed)>,
        marked: Vec<UnitId>,
        removed: Vec<UnitId>,
        spawned_count: i32,
    }

    /// Выполняет одно действие над хранилищем и над моделью.
    /// `kind` выбирает действие, `pick` выбирает юнит для отметки.
    /// Возвращает идентификатор, если действие создало юнит.
    fn apply(store: &mut UnitStore, model: &mut Model, kind: u8, pick: usize) -> Option<UnitId> {
        if kind <= 2 {
            // Создать. У каждого юнита своя позиция, чтобы заметить,
            // если хранилище перепутает юниты.
            let position = Vec2Fixed::from_ints(model.spawned_count, -model.spawned_count);
            model.spawned_count += 1;
            let unit_id = store.spawn(position);
            model.alive.push((unit_id, position));
            return Some(unit_id);
        }
        if kind <= 4 {
            // Отметить случайного живого (можно и повторно).
            if !model.alive.is_empty() {
                let (unit_id, _) = model.alive[pick % model.alive.len()];
                store.mark_for_removal(unit_id);
                if !model.marked.contains(&unit_id) {
                    model.marked.push(unit_id);
                }
            }
            return None;
        }
        // Конец тика: удалить отмеченных.
        store.remove_marked();
        for unit_id in &model.marked {
            model.alive.retain(|(alive_id, _)| alive_id != unit_id);
            model.removed.push(*unit_id);
        }
        model.marked.clear();
        None
    }

    fn empty_model() -> Model {
        Model {
            alive: Vec::new(),
            marked: Vec::new(),
            removed: Vec::new(),
            spawned_count: 0,
        }
    }

    /// Случайная последовательность действий: пары (вид, выбор юнита).
    fn actions() -> impl Strategy<Value = Vec<(u8, usize)>> {
        prop::collection::vec((0u8..6, any::<usize>()), 0..200)
    }

    proptest! {
        /// После каждого действия хранилище согласовано с моделью.
        #[test]
        fn store_matches_the_model(actions in actions()) {
            let mut store = UnitStore::new();
            let mut model = empty_model();
            for (kind, pick) in actions {
                apply(&mut store, &mut model, kind, pick);

                prop_assert_eq!(store.len(), model.alive.len());
                // Каждый живой находится, и это именно он.
                for (unit_id, position) in &model.alive {
                    let unit = store.get(*unit_id).expect("живой юнит должен находиться");
                    prop_assert_eq!(unit.id, *unit_id);
                    prop_assert_eq!(unit.position, *position);
                }
                // Удалённый не находится никогда, даже если ячейку заняли.
                for unit_id in &model.removed {
                    prop_assert_eq!(store.get(*unit_id), None);
                }
                // Обход: ровно живые, строго по возрастанию индекса.
                let ids = ids_in_order(&store);
                prop_assert_eq!(ids.len(), model.alive.len());
                for pair in ids.windows(2) {
                    prop_assert!(pair[0].index < pair[1].index);
                }
            }
        }

        /// Одни и те же действия дважды дают одни и те же идентификаторы.
        #[test]
        fn same_actions_give_same_ids(actions in actions()) {
            let mut first_ids = Vec::new();
            let mut store = UnitStore::new();
            let mut model = empty_model();
            for (kind, pick) in &actions {
                first_ids.push(apply(&mut store, &mut model, *kind, *pick));
            }

            let mut second_ids = Vec::new();
            let mut store = UnitStore::new();
            let mut model = empty_model();
            for (kind, pick) in &actions {
                second_ids.push(apply(&mut store, &mut model, *kind, *pick));
            }
            prop_assert_eq!(first_ids, second_ids);
        }
    }
}
