use crate::components::Navigation;
use dioxus::prelude::*;

#[component]
pub fn ApiDocs() -> Element {
    let base_url = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .map(|hostname| {
            if hostname == "localhost" || hostname == "127.0.0.1" {
                "http://localhost:3000/api".to_string()
            } else {
                "/api".to_string()
            }
        })
        .unwrap_or_else(|| "/api".to_string());

    rsx! {
        Navigation {}

        main { class: "min-h-screen bg-gray-900 py-12",
            div { class: "container mx-auto px-6",
                div { class: "text-center mb-12",
                    h1 { class: "text-4xl md:text-5xl font-bold text-gray-100 mb-6", "REST API" }
                    p { class: "text-xl text-gray-400 max-w-3xl mx-auto",
                        "JSON API for querying package metadata"
                    }
                }

                div { class: "max-w-4xl mx-auto",
                    div { class: "bg-gray-800 rounded-2xl shadow-xl overflow-hidden border border-gray-700",
                        div { class: "bg-gray-700 p-6 border-b border-gray-600",
                            h2 { class: "text-2xl font-bold text-gray-100", "Endpoints" }
                            div { class: "mt-3 flex items-center gap-2",
                                span { class: "text-sm text-gray-400", "Base URL:" }
                                code { class: "bg-gray-900 px-3 py-1 rounded text-blue-400 font-mono text-sm",
                                    "{base_url}"
                                }
                            }
                        }

                        div { class: "p-8 space-y-8",
                            // Stats endpoint
                            div { class: "border-l-4 border-blue-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-blue-900 text-blue-300 rounded-full text-sm font-medium", "GET" }
                                    code { class: "text-lg font-mono text-gray-200", "/stats" }
                                }
                                p { class: "text-gray-400 mb-3", "Get real-time database statistics" }
                            }

                            // Packages list
                            div { class: "border-l-4 border-blue-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-blue-900 text-blue-300 rounded-full text-sm font-medium", "GET" }
                                    code { class: "text-lg font-mono text-gray-200", "/packages" }
                                }
                                p { class: "text-gray-400 mb-3", "List packages (with optional search/filtering)" }
                                div { class: "text-sm text-gray-400",
                                    strong { "Query Parameters:" }
                                    " search, page, limit, tag"
                                }
                            }

                            // Package detail
                            div { class: "border-l-4 border-blue-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-blue-900 text-blue-300 rounded-full text-sm font-medium", "GET" }
                                    code { class: "text-lg font-mono text-gray-200", "/packages/{{id}}" }
                                }
                                p { class: "text-gray-400", "Get package details by ID" }
                            }

                            // Register
                            div { class: "border-l-4 border-purple-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-purple-900 text-purple-300 rounded-full text-sm font-medium", "POST" }
                                    code { class: "text-lg font-mono text-gray-200", "/auth/register" }
                                }
                                p { class: "text-gray-400 mb-3", "Register new user (returns JWT token)" }
                                div { class: "text-sm text-gray-400",
                                    strong { "Body:" }
                                    " username, email, password"
                                }
                            }

                            // Login
                            div { class: "border-l-4 border-purple-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-purple-900 text-purple-300 rounded-full text-sm font-medium", "POST" }
                                    code { class: "text-lg font-mono text-gray-200", "/auth/login" }
                                }
                                p { class: "text-gray-400 mb-3", "Authenticate (returns JWT token)" }
                                div { class: "text-sm text-gray-400",
                                    strong { "Body:" }
                                    " email, password"
                                }
                            }

                            // Subscriptions (Protected)
                            div { class: "border-l-4 border-green-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-green-900 text-green-300 rounded-full text-sm font-medium", "GET" }
                                    code { class: "text-lg font-mono text-gray-200", "/users/subscriptions" }
                                    span { class: "px-2 py-1 bg-yellow-900 text-yellow-300 rounded text-xs", "AUTH" }
                                }
                                p { class: "text-gray-400", "Get user's subscriptions" }
                            }

                            // Timeline (Protected)
                            div { class: "border-l-4 border-green-500 pl-6",
                                div { class: "flex items-center gap-3 mb-3",
                                    span { class: "px-3 py-1 bg-green-900 text-green-300 rounded-full text-sm font-medium", "GET" }
                                    code { class: "text-lg font-mono text-gray-200", "/users/timeline" }
                                    span { class: "px-2 py-1 bg-yellow-900 text-yellow-300 rounded text-xs", "AUTH" }
                                }
                                p { class: "text-gray-400", "Get personal timeline (paginated)" }
                            }

                            // Notes
                            div { class: "mt-8 p-4 bg-gray-900 rounded-lg border border-gray-700",
                                h3 { class: "text-sm font-semibold text-gray-300 mb-2", "Notes" }
                                ul { class: "text-sm text-gray-400 space-y-1",
                                    li { "• All responses are JSON" }
                                    li { "• JWT tokens expire after 7 days" }
                                    li { "• Protected endpoints return 401 if token is missing/invalid" }
                                    li { "• Configure base URL via FOSSDB_API_URL environment variable" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
