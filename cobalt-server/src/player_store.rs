use std::sync::atomic::{AtomicBool, Ordering};

pub struct Position {
    x: f64,
    y: f64,
    z: f64,
}

pub struct Rotation {
    yaw: f32,
    pitch: f32,
}

pub struct Player {
    position: Position,
    rotation: Rotation,
}

pub struct PlayerStore {
    players: Vec<u8>,
    pub dirty: Vec<AtomicBool>,
}

impl PlayerStore {
    pub fn set_position(&mut self, id: i32, position: Position) {
        self.set_dirty(id);
        todo!()
    }

    pub fn set_rotation(&mut self, id: i32, rotation: Rotation) {
        self.set_dirty(id);
        todo!()
    }

    pub fn set_position_and_rotation(&mut self, id: i32, position: Position, rotation: Rotation) {
        self.set_dirty(id);
        todo!()
    }

    pub fn leave(&mut self, id: i32) {
        // remove from vec
        todo!()
    }

    #[inline]
    fn set_dirty(&self, id: i32) {
        self.dirty[id as usize].store(true, Ordering::Release);
    }

    pub fn take_dirty(&self) -> impl Iterator<Item = usize> + '_ {
        self.dirty
            .iter()
            .enumerate()
            .filter_map(|(i, flag)| flag.swap(false, Ordering::AcqRel).then_some(i))
    }
}
