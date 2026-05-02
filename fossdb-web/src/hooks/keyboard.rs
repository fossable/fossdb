use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, KeyboardEvent};

#[derive(Clone, Copy, PartialEq)]
pub struct KeyPress {
    pub key: &'static str,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

pub fn use_keyboard_shortcut<F>(shortcut: KeyPress, callback: F)
where
    F: Fn() + 'static + Clone,
{
    use_effect(move || {
        let window = window().expect("no global `window` exists");
        let callback_clone = callback.clone();

        let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let matches = event.key() == shortcut.key
                && event.ctrl_key() == shortcut.ctrl
                && event.shift_key() == shortcut.shift
                && event.alt_key() == shortcut.alt;

            if matches {
                event.prevent_default();
                callback_clone();
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        window
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .expect("failed to add event listener");

        closure.forget();
    });
}
