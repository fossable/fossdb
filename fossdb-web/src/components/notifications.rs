use crate::hooks::{Notification, NotificationState, NotificationType};
use dioxus::prelude::*;

#[component]
pub fn NotificationContainer() -> Element {
    let state = use_context::<Signal<NotificationState>>();
    let notifications = state.read().notifications.clone();

    rsx! {
        div { class: "fixed top-4 right-4 z-50 space-y-2",
            for (index, notification) in notifications.iter().enumerate() {
                NotificationToast {
                    key: "{notification.id}",
                    notification: notification.clone(),
                    index: index
                }
            }
        }
    }
}

#[component]
fn NotificationToast(notification: Notification, index: usize) -> Element {
    let color_class = match notification.notification_type {
        NotificationType::Success => "bg-green-500 text-white",
        NotificationType::Error => "bg-red-500 text-white",
        NotificationType::Info => "bg-blue-500 text-white",
        NotificationType::Warning => "bg-yellow-500 text-black",
    };

    let animation_class = "animate-slideIn";
    let top_offset = index * 72;

    rsx! {
        div {
            class: "px-6 py-4 rounded-lg shadow-lg transition-all duration-300 {color_class} {animation_class}",
            style: "margin-top: {top_offset}px;",
            "{notification.message}"
        }
    }
}
