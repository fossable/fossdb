use dioxus::prelude::*;
use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct WebSocketState {
    pub connected: bool,
    pub reconnecting: bool,
}

pub fn use_websocket<T, F>(url: String, on_message: F)
where
    T: for<'de> serde::Deserialize<'de> + 'static,
    F: FnMut(T) + 'static,
{
    let mut state = use_signal(|| WebSocketState {
        connected: false,
        reconnecting: false,
    });

    let on_message = Rc::new(RefCell::new(on_message));

    use_effect(move || {
        let url = url.clone();
        let on_message = on_message.clone();

        spawn(async move {
            loop {
                state.write().reconnecting = true;

                match WebSocket::open(&url) {
                    Ok(mut ws) => {
                        state.write().connected = true;
                        state.write().reconnecting = false;

                        while let Some(msg) = ws.next().await {
                            match msg {
                                Ok(Message::Text(text)) => {
                                    if let Ok(data) = serde_json::from_str::<T>(&text) {
                                        (on_message.borrow_mut())(data);
                                    }
                                }
                                Ok(Message::Bytes(_)) => {}
                                Err(_) => {
                                    state.write().connected = false;
                                    break;
                                }
                            }
                        }

                        state.write().connected = false;
                    }
                    Err(_) => {
                        state.write().connected = false;
                        state.write().reconnecting = false;
                    }
                }

                gloo_timers::future::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    });
}
