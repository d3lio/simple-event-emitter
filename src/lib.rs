use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

type Id = u64;

#[derive(Debug)]
pub enum Error {
    ListenerNotFound,
    UnknownEvent
}

pub struct Listener<P> {
    id: Id,
    closure: Box<Fn(P)>
}

impl<P> Listener<P> {
    pub fn new<F>(id: Id, f: F) -> Self where F: Fn(P) + 'static {
        Self {
            id,
            closure: Box::new(f)
        }
    }
}

pub struct EventEmitter<E, P> where E: Eq + Hash, P: Clone {
    next_id: Id,
    map: HashMap<E, Vec<Listener<P>>>
}

impl<E, P> EventEmitter<E, P> where E: Eq + Hash, P: Clone {
    pub fn new() -> Self {
        Self {
            next_id: Id::default(),
            map: HashMap::new()
        }
    }

    pub fn on<F>(&mut self, event: E, listener: F) -> Id where F: Fn(P) + 'static {
        let id = self.next_id;
        let listeners = self.map.entry(event).or_insert(Vec::new());

        listeners.push(Listener::new(self.next_id, listener));

        self.next_id += 1;
        id
    }

    pub fn off(&mut self, id: Id) -> Result<(), Error> {
        for (_, listeners) in self.map.iter_mut() {
            let position = listeners.iter().position(|x| x.id == id);

            if let Some(idx) = position {
                listeners.remove(idx);
                return Ok(());
            }
        }

        Err(Error::ListenerNotFound)
    }

    pub fn emit<B>(&self, event: &B, payload: P) -> Result<(), Error>
    where E: Borrow<B>, B: ?Sized + Hash + Eq {
        match self.map.get(event) {
            Some(listeners) => {
                listeners.iter().for_each(|f| (f.closure)(payload.clone()));
                Ok(())
            },
            None => Err(Error::UnknownEvent)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_register_listeners() {
        let mut emitter = EventEmitter::new();
        emitter.on("event", |_: i32| {});
        emitter.on("event", |_: i32| {});
    }

    #[test]
    fn can_unregister_listeners() {
        let mut emitter = EventEmitter::new();
        let id = emitter.on("event", |_: i32| {});
        emitter.on("event", |_: i32| {});

        assert_eq!(emitter.map.get("event").unwrap().len(), 2);

        assert!(emitter.off(id).is_ok());

        assert_eq!(emitter.map.get("event").unwrap().len(), 1);
    }

    #[test]
    fn can_call_listeners() {
        use std::sync::mpsc::{channel, TryRecvError};

        let (sx, rx) = channel();
        let sx1 = sx.clone();
        let sx2 = sx.clone();

        let mut emitter = EventEmitter::new();
        emitter.on("event", move |payload: i32| sx1.send(payload).unwrap());
        emitter.on("event", move |payload: i32| sx2.send(payload).unwrap());

        emitter.emit("event", 3).unwrap();

        assert_eq!(rx.recv(), Ok(3));
        assert_eq!(rx.recv(), Ok(3));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn can_pass_payload_by_reference() {
        use std::sync::mpsc::{channel, TryRecvError};

        let (sx, rx) = channel();
        let sx1 = sx.clone();
        let sx2 = sx.clone();

        let mut emitter = EventEmitter::new();
        emitter.on("event1", move |payload: &i32| sx1.send(payload).unwrap());
        emitter.on("event2", move |payload: &i32| sx2.send(payload).unwrap());

        emitter.emit("event1", &1).unwrap();
        emitter.emit("event1", &2).unwrap();

        assert_eq!(rx.recv(), Ok(&1));
        assert_eq!(rx.recv(), Ok(&2));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn can_invoke_emit_as_hash_map_get() {
        use std::sync::mpsc::{channel, TryRecvError};

        let (sx, rx) = channel();

        let mut emitter = EventEmitter::new();
        emitter.on(String::from("event1"), move |payload: &i32| sx.send(payload).unwrap());

        emitter.emit("event1", &1).unwrap();

        assert_eq!(rx.recv(), Ok(&1));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }
}
