use dioxus::prelude::*;
use std::collections::VecDeque;

#[derive(Clone, PartialEq, Debug)]
pub enum NotificationType {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Notification {
    pub id: usize,
    pub message: String,
    pub notification_type: NotificationType,
}

#[derive(Clone, PartialEq)]
pub struct NotificationState {
    pub notifications: VecDeque<Notification>,
    next_id: usize,
}

impl Default for NotificationState {
    fn default() -> Self {
        Self {
            notifications: VecDeque::new(),
            next_id: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct NotificationContext {
    state: Signal<NotificationState>,
}

impl NotificationContext {
    pub fn show(&mut self, message: String, notification_type: NotificationType) {
        let id = {
            let mut state = self.state.write();
            let id = state.next_id;
            state.next_id += 1;

            state.notifications.push_back(Notification {
                id,
                message,
                notification_type,
            });

            if state.notifications.len() > 3 {
                state.notifications.pop_front();
            }
            id
        };

        let mut state_clone = self.state;
        spawn(async move {
            gloo_timers::future::sleep(std::time::Duration::from_secs(3)).await;
            state_clone.write().notifications.retain(|n| n.id != id);
        });
    }

    pub fn success(&mut self, message: String) {
        self.show(message, NotificationType::Success);
    }

    pub fn error(&mut self, message: String) {
        self.show(message, NotificationType::Error);
    }

    pub fn info(&mut self, message: String) {
        self.show(message, NotificationType::Info);
    }

    pub fn warning(&mut self, message: String) {
        self.show(message, NotificationType::Warning);
    }
}

pub fn use_notifications() -> NotificationContext {
    let state = use_context::<Signal<NotificationState>>();
    NotificationContext { state }
}
