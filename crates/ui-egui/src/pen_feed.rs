//! Pen pressure from a platform tablet reader, for pens the windowing layer reports as a mouse
//! (X11 today). The desktop app writes it from its reader thread; [`crate::viewport::pen_pressure`]
//! reads it when no touch force arrives.

use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

/// Whether the platform tablet reader runs, as About and `ui.inspect` report it.
#[derive(Clone, Debug, PartialEq)]
pub enum TabletReader {
    /// Reading pen pressure through the named interface.
    Reading(&'static str),
    /// Pens paint like a mouse, for this reason.
    Unavailable(String),
}

/// The latest pressure (0..1) while a tablet pen is in use, `None` once another pointer moves,
/// and the reader's status. Clones share both.
#[derive(Clone)]
pub struct PenFeed {
    pressure: Arc<Mutex<Option<f32>>>,
    reader: Arc<Mutex<TabletReader>>,
}

impl PenFeed {
    pub fn new(reader: TabletReader) -> Self {
        Self { pressure: Arc::default(), reader: Arc::new(Mutex::new(reader)) }
    }

    pub fn set(&self, pressure: Option<f32>) {
        let pressure = pressure.map(|p| if p.is_finite() { p.clamp(0.0, 1.0) } else { 1.0 });
        *self.pressure.lock().unwrap_or_else(PoisonError::into_inner) = pressure;
    }

    pub fn get(&self) -> Option<f32> {
        *self.pressure.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_reader(&self, reader: TabletReader) {
        *self.reader.lock().unwrap_or_else(PoisonError::into_inner) = reader;
    }

    pub fn reader(&self) -> TabletReader {
        self.reader.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Makes this the feed `pen_pressure` reads in `ctx`.
    pub fn install(&self, ctx: &egui::Context) {
        ctx.data_mut(|data| data.insert_temp(feed_id(), self.clone()));
    }

    pub fn installed(ctx: &egui::Context) -> Option<Self> {
        ctx.data(|data| data.get_temp(feed_id()))
    }

    /// `ui.inspect`'s report: `null` without a tablet reader (pressure then comes only as touch force)
    pub fn inspect(ctx: &egui::Context) -> Value {
        let Some(feed) = Self::installed(ctx) else { return Value::Null };
        match feed.reader() {
            TabletReader::Reading(interface) => json!({"available": true, "reader": interface, "pressure": feed.get()}),
            TabletReader::Unavailable(reason) => json!({"available": false, "reason": reason}),
        }
    }
}

fn feed_id() -> egui::Id {
    egui::Id::unique("tc_pen_feed")
}
