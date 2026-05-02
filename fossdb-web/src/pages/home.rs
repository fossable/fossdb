use crate::api::types::*;
use crate::api::ApiClient;
use crate::hooks::{use_auth, use_time_ago, use_websocket};
use dioxus::prelude::*;
use std::collections::HashSet;

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    let mut stats = use_signal(|| None::<DatabaseStats>);
    let mut timeline_events = use_signal(|| Vec::<TimelineEvent>::new());
    let mut latest_packages = use_signal(|| Vec::<Package>::new());
    let mut loading = use_signal(|| true);
    let mut timeline_offset = use_signal(|| 0);
    let mut timeline_total = use_signal(|| 0);
    let mut timeline_loading = use_signal(|| false);
    let mut displayed_event_ids = use_signal(|| HashSet::<u64>::new());

    let token = auth.token();
    let is_authenticated = auth.is_authenticated();

    // Load initial data
    let token_for_effect = token.clone();
    use_effect(move || {
        let token_clone = token_for_effect.clone();
        spawn(async move {
            let client = ApiClient::new().with_token(token_clone.clone());

            if let Ok(db_stats) = client.get_stats().await {
                stats.set(Some(db_stats));
            }

            if is_authenticated {
                if let Ok(timeline) = client.get_timeline(0, 20).await {
                    let mut ids = HashSet::new();
                    for event in &timeline.events {
                        ids.insert(event.id);
                    }
                    displayed_event_ids.set(ids);
                    timeline_events.set(timeline.events);
                    timeline_total.set(timeline.total);
                    timeline_offset.set(20);
                }
            } else {
                if let Ok(response) = client.get_packages(None, 1, 6).await {
                    latest_packages.set(response.packages);
                }
            }

            loading.set(false);
        });
    });

    // WebSocket for real-time timeline updates
    // - Unauthenticated users receive global timeline events
    // - Authenticated users receive personalized timeline events
    {
        let ws_url = if cfg!(debug_assertions) {
            "ws://localhost:3000/ws/timeline".to_string()
        } else {
            format!(
                "ws://{}/ws/timeline",
                web_sys::window()
                    .and_then(|w| w.location().host().ok())
                    .unwrap_or_else(|| "localhost:3000".to_string())
            )
        };

        use_websocket::<WebSocketMessage, _>(ws_url, move |msg: WebSocketMessage| {
            match msg {
                WebSocketMessage::TimelineEvent { event } => {
                    let event_id = event.id;
                    if !displayed_event_ids.read().contains(&event_id) {
                        displayed_event_ids.write().insert(event_id);
                        timeline_events.write().insert(0, event);
                    }
                }
                _ => {
                    // Ignore other message types (Ping, Pong, Auth)
                }
            }
        });
    }

    let load_more_timeline = move |_| {
        if timeline_loading() {
            return;
        }

        let offset = timeline_offset();
        let token_clone = token.clone();

        spawn(async move {
            timeline_loading.set(true);
            let client = ApiClient::new().with_token(token_clone);

            if let Ok(timeline) = client.get_timeline(offset, 20).await {
                let mut events = timeline_events.write();
                let mut ids = displayed_event_ids.write();

                for event in timeline.events {
                    if !ids.contains(&event.id) {
                        ids.insert(event.id);
                        events.push(event);
                    }
                }

                timeline_offset.set(offset + 20);
            }

            timeline_loading.set(false);
        });
    };

    rsx! {
        main { class: "relative",
            // Hero Section
            section { class: "hero-gradient relative overflow-hidden",
                div { class: "container mx-auto px-6 py-20 relative z-10",
                    div { class: "text-white max-w-4xl",
                        h1 { class: "text-4xl md:text-6xl font-bold mb-4 leading-tight",
                            "FossDB"
                        }
                        p { class: "text-lg md:text-xl mb-8 text-gray-300 leading-relaxed",
                            "A self-hosted database for tracking open source software packages. "
                            "Scrapes package metadata from registries like crates.io and provides "
                            "a queryable REST API for dependency analysis."
                        }

                        // Real-time stats
                        if let Some(db_stats) = stats() {
                            div { class: "grid grid-cols-2 md:grid-cols-4 gap-4 mb-8",
                                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                                    div { class: "text-gray-400 text-sm", "Packages" }
                                    div { class: "text-2xl font-bold text-blue-400", "{db_stats.total_packages}" }
                                }
                                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                                    div { class: "text-gray-400 text-sm", "Versions" }
                                    div { class: "text-2xl font-bold text-purple-400", "{db_stats.total_versions}" }
                                }
                                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                                    div { class: "text-gray-400 text-sm", "Users" }
                                    div { class: "text-2xl font-bold text-green-400", "{db_stats.total_users}" }
                                }
                                div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700",
                                    div { class: "text-gray-400 text-sm", "CVEs" }
                                    div { class: "text-2xl font-bold text-red-400", "{db_stats.total_vulnerabilities}" }
                                }
                            }
                        }
                    }
                }
            }

            // Timeline Section
            section { class: "py-24 bg-gray-900",
                div { class: "container mx-auto px-6",
                    div { class: "text-center mb-16",
                        if is_authenticated {
                            h2 { class: "text-4xl font-bold text-gray-100 mb-6", "Your Timeline" }
                            p { class: "text-xl text-gray-300", "Updates from packages you follow" }
                        } else {
                            h2 { class: "text-4xl font-bold text-gray-100 mb-6", "Global Timeline" }
                            p { class: "text-xl text-gray-300", "Real-time updates from the open source ecosystem" }
                        }
                    }

                    div { class: "max-w-4xl mx-auto space-y-4",
                        if loading() {
                            div { class: "flex justify-center py-12",
                                div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                            }
                        } else {
                            for event in timeline_events().iter() {
                                TimelineEventCard { event: event.clone() }
                            }

                            // Load More button
                            if timeline_events().len() < timeline_total() as usize {
                                div { class: "flex justify-center mt-8",
                                    button {
                                        class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                                        disabled: timeline_loading(),
                                        onclick: load_more_timeline,
                                        if timeline_loading() {
                                            "Loading..."
                                        } else {
                                            "Load More"
                                        }
                                    }
                                }
                            }

                            if timeline_events().is_empty() {
                                div { class: "text-center py-12",
                                    if is_authenticated {
                                        p { class: "text-gray-400 text-lg", "No timeline events yet. Subscribe to packages to see updates here!" }
                                    } else {
                                        p { class: "text-gray-400 text-lg", "Waiting for new releases..." }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TimelineEventCard(event: TimelineEvent) -> Element {
    let time_ago = use_time_ago(event.created_at);

    let (icon_class, icon_color) = match event.event_type {
        TimelineEventType::PackageAdded => ("M12 4v16m8-8H4", "text-green-400"),
        TimelineEventType::NewRelease => ("M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z", "text-blue-400"),
        TimelineEventType::SecurityAlert => (
            "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
            "text-red-400",
        ),
        TimelineEventType::PackageUpdated => ("M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z", "text-gray-400"),
    };

    rsx! {
        div { class: "bg-gray-800 rounded-lg p-6 border border-gray-700",
            div { class: "flex items-start space-x-4",
                div { class: "flex-shrink-0",
                    svg {
                        class: "w-6 h-6 {icon_color}",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "{icon_class}"
                        }
                    }
                }
                div { class: "flex-1",
                    h3 { class: "text-lg font-semibold text-gray-100 mb-2",
                        "{event.package_name}"
                    }
                    p { class: "text-gray-400", "{event.message}" }
                    if let Some(v) = &event.version {
                        p { class: "text-sm text-blue-400 mt-1", "Version: {v}" }
                    }
                }
                div { class: "text-sm text-gray-500",
                    "{time_ago()}"
                }
            }
        }
    }
}
