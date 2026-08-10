//! WebAssembly bindings for the interactive rules engine.
//!
//! Compiled only under the `wasm` feature so the CLI build stays free of
//! `wasm-bindgen`. The browser owns presentation and randomness (it
//! supplies the seed); everything about what is legal and what a position
//! scores lives on this side of the boundary.

use wasm_bindgen::prelude::*;

use crate::game::rng::Rng;
use crate::game::{self, Action, Game};

/// A game in progress, held by the page.
#[wasm_bindgen]
pub struct Session {
    game: Box<dyn Game>,
    rng: Rng,
}

#[wasm_bindgen]
impl Session {
    /// Start a game of the discipline `key`, seeding the dice with `seed`.
    ///
    /// # Errors
    ///
    /// Returns an error if `key` names no playable discipline.
    #[wasm_bindgen(constructor)]
    pub fn new(key: &str, seed: f64) -> Result<Self, JsValue> {
        // JS numbers reach us as f64; the page passes an integer below
        // 2^53 so this round-trips exactly.
        let mut rng = Rng::new(seed.abs() as u64);
        let game = game::start(key, &mut rng).ok_or_else(|| {
            JsValue::from_str(&format!("no playable engine for {key}"))
        })?;
        Ok(Self { game, rng })
    }

    /// The current position, as JSON matching `game::View`.
    ///
    /// # Errors
    ///
    /// Returns an error if the view fails to serialise.
    pub fn view(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.game.view())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Play `action`, given as the JSON of a `game::Action`. Returns
    /// `true` if the move was legal and was applied.
    ///
    /// # Errors
    ///
    /// Returns an error if `action` is not valid action JSON.
    pub fn apply(&mut self, action: &str) -> Result<bool, JsValue> {
        let action: Action = serde_json::from_str(action)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(self.game.apply(&action, &mut self.rng))
    }
}

/// Playable disciplines as JSON `[{"key":..,"name":..}]`, in rulebook
/// order — what the menu lists.
///
/// # Panics
///
/// Panics only if serialising a list of string literals fails.
#[wasm_bindgen]
#[must_use]
pub fn catalogue() -> String {
    let items: Vec<_> = game::catalogue()
        .into_iter()
        .map(|(key, name)| serde_json::json!({ "key": key, "name": name }))
        .collect();
    serde_json::to_string(&items).expect("string list serialises")
}

/// Keys of the disciplines that can be played interactively, as a JSON
/// array of strings.
///
/// # Panics
///
/// Panics only if serialising a list of string literals fails.
#[wasm_bindgen]
#[must_use]
pub fn playable() -> String {
    serde_json::to_string(&game::playable()).expect("string list serialises")
}
