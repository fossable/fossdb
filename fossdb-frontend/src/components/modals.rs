use crate::api::ApiClient;
use crate::hooks::{use_auth, use_notifications};
use dioxus::prelude::*;

fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.') && email.len() > 5
}

#[component]
pub fn LoginModal(show: Signal<bool>) -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut email_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut email_touched = use_signal(|| false);
    let auth = use_auth();
    let notif = use_notifications();

    let mut validate_email = move || {
        let email_val = email();
        if email_touched() {
            if email_val.is_empty() {
                email_error.set(Some("Email is required".to_string()));
            } else if !is_valid_email(&email_val) {
                email_error.set(Some("Please enter a valid email address".to_string()));
            } else {
                email_error.set(None);
            }
        }
    };

    let handle_submit = move |evt: Event<FormData>| {
        evt.prevent_default();

        let email_val = email();
        let password_val = password();

        // Client-side validation
        if !is_valid_email(&email_val) {
            email_touched.set(true);
            if email_val.is_empty() {
                email_error.set(Some("Email is required".to_string()));
            } else {
                email_error.set(Some("Please enter a valid email address".to_string()));
            }
            return;
        }

        if password_val.is_empty() {
            form_error.set(Some("Password is required".to_string()));
            return;
        }

        let mut auth_copy = auth;
        let mut notif_copy = notif;

        spawn(async move {
            let client = ApiClient::new();
            match client.login(email_val, password_val).await {
                Ok(response) => {
                    auth_copy.login(response.token, response.user);
                    notif_copy.success("Logged in successfully".to_string());
                    show.set(false);
                    // Reset form
                    email.set(String::new());
                    password.set(String::new());
                    email_error.set(None);
                    form_error.set(None);
                    email_touched.set(false);
                }
                Err(e) => {
                    let err_msg = e.as_string().unwrap_or_else(|| "Login failed".to_string());
                    form_error.set(Some(err_msg));
                }
            }
        });
    };

    if !show() {
        return rsx! { div {} };
    }

    rsx! {
        div { class: "fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
            onclick: move |_| {
                show.set(false);
                email.set(String::new());
                password.set(String::new());
                email_error.set(None);
                form_error.set(None);
                email_touched.set(false);
            },
            div { class: "bg-gray-800 rounded-2xl w-full max-w-md shadow-2xl border border-gray-700",
                onclick: move |evt| evt.stop_propagation(),
                div { class: "p-8",
                    h3 { class: "text-2xl font-bold text-gray-100 mb-6 text-center", "Welcome Back" }

                    form {
                        onsubmit: handle_submit,

                        div { class: "mb-4",
                            label { class: "block text-gray-300 font-medium mb-2", "Email" }
                            input {
                                r#type: "email",
                                class: if email_error().is_some() {
                                    "w-full p-3 bg-gray-700 border-2 border-red-500 rounded-lg focus:ring-2 focus:ring-red-400 text-gray-100 placeholder-gray-400"
                                } else {
                                    "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100 placeholder-gray-400"
                                },
                                placeholder: "Enter your email address",
                                value: "{email}",
                                oninput: move |evt| {
                                    email.set(evt.value());
                                    if email_touched() {
                                        validate_email();
                                    }
                                    if form_error().is_some() {
                                        form_error.set(None);
                                    }
                                },
                                onblur: move |_| {
                                    email_touched.set(true);
                                    validate_email();
                                },
                                required: true
                            }
                            if let Some(err) = email_error() {
                                div { class: "mt-1 text-sm text-red-400", "{err}" }
                            }
                        }

                        div { class: "mb-6",
                            label { class: "block text-gray-300 font-medium mb-2", "Password" }
                            input {
                                r#type: "password",
                                class: "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100 placeholder-gray-400",
                                placeholder: "Enter your password",
                                value: "{password}",
                                oninput: move |evt| {
                                    password.set(evt.value());
                                    if form_error().is_some() {
                                        form_error.set(None);
                                    }
                                },
                                required: true
                            }
                        }

                        if let Some(err) = form_error() {
                            div { class: "mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg animate-shake",
                                div { class: "flex items-center space-x-3",
                                    svg { class: "w-5 h-5 text-red-400 flex-shrink-0", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                    div { class: "text-sm text-red-300", "{err}" }
                                }
                            }
                        }

                        div { class: "flex gap-3",
                            button {
                                r#type: "submit",
                                class: "flex-1 bg-gradient-to-r from-blue-500 to-purple-600 text-white p-3 rounded-lg font-medium hover:from-blue-600 hover:to-purple-700 transition-all",
                                "Login"
                            }
                            button {
                                r#type: "button",
                                class: "flex-1 bg-gray-600 text-gray-200 p-3 rounded-lg font-medium hover:bg-gray-500 transition-all",
                                onclick: move |_| {
                                    show.set(false);
                                    email.set(String::new());
                                    password.set(String::new());
                                    email_error.set(None);
                                    form_error.set(None);
                                    email_touched.set(false);
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn RegisterModal(show: Signal<bool>) -> Element {
    let mut username = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut username_error = use_signal(|| None::<String>);
    let mut email_error = use_signal(|| None::<String>);
    let mut password_error = use_signal(|| None::<String>);
    let mut form_error = use_signal(|| None::<String>);
    let mut username_touched = use_signal(|| false);
    let mut email_touched = use_signal(|| false);
    let mut password_touched = use_signal(|| false);
    let auth = use_auth();
    let notif = use_notifications();

    let mut validate_username = move || {
        let username_val = username();
        if username_touched() {
            if username_val.is_empty() {
                username_error.set(Some("Username is required".to_string()));
            } else if username_val.len() < 3 {
                username_error.set(Some("Username must be at least 3 characters".to_string()));
            } else {
                username_error.set(None);
            }
        }
    };

    let mut validate_email_fn = move || {
        let email_val = email();
        if email_touched() {
            if email_val.is_empty() {
                email_error.set(Some("Email is required".to_string()));
            } else if !is_valid_email(&email_val) {
                email_error.set(Some("Please enter a valid email address".to_string()));
            } else {
                email_error.set(None);
            }
        }
    };

    let mut validate_password = move || {
        let password_val = password();
        if password_touched() {
            if password_val.is_empty() {
                password_error.set(Some("Password is required".to_string()));
            } else if password_val.len() < 6 {
                password_error.set(Some("Password must be at least 6 characters".to_string()));
            } else {
                password_error.set(None);
            }
        }
    };

    let handle_submit = move |evt: Event<FormData>| {
        evt.prevent_default();

        let username_val = username();
        let email_val = email();
        let password_val = password();

        // Mark all as touched and validate
        username_touched.set(true);
        email_touched.set(true);
        password_touched.set(true);

        // Manual validation for submit
        if username_val.is_empty() || username_val.len() < 3 {
            username_error.set(Some("Username must be at least 3 characters".to_string()));
            return;
        }
        if email_val.is_empty() || !is_valid_email(&email_val) {
            email_error.set(Some("Please enter a valid email address".to_string()));
            return;
        }
        if password_val.is_empty() || password_val.len() < 6 {
            password_error.set(Some("Password must be at least 6 characters".to_string()));
            return;
        }

        let mut auth_copy = auth;
        let mut notif_copy = notif;

        spawn(async move {
            let client = ApiClient::new();
            match client.register(username_val, email_val, password_val).await {
                Ok(response) => {
                    auth_copy.login(response.token, response.user);
                    notif_copy.success("Account created successfully".to_string());
                    show.set(false);
                    // Reset form
                    username.set(String::new());
                    email.set(String::new());
                    password.set(String::new());
                    username_error.set(None);
                    email_error.set(None);
                    password_error.set(None);
                    form_error.set(None);
                    username_touched.set(false);
                    email_touched.set(false);
                    password_touched.set(false);
                }
                Err(e) => {
                    let err_msg = e
                        .as_string()
                        .unwrap_or_else(|| "Registration failed".to_string());
                    form_error.set(Some(err_msg));
                }
            }
        });
    };

    if !show() {
        return rsx! { div {} };
    }

    rsx! {
        div { class: "fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
            onclick: move |_| {
                show.set(false);
                username.set(String::new());
                email.set(String::new());
                password.set(String::new());
                username_error.set(None);
                email_error.set(None);
                password_error.set(None);
                form_error.set(None);
                username_touched.set(false);
                email_touched.set(false);
                password_touched.set(false);
            },
            div { class: "bg-gray-800 rounded-2xl w-full max-w-md shadow-2xl border border-gray-700",
                onclick: move |evt| evt.stop_propagation(),
                div { class: "p-8",
                    h3 { class: "text-2xl font-bold text-gray-100 mb-6 text-center", "Join FossDB" }

                    form {
                        onsubmit: handle_submit,

                        div { class: "mb-4",
                            label { class: "block text-gray-300 font-medium mb-2", "Username" }
                            input {
                                r#type: "text",
                                class: if username_error().is_some() {
                                    "w-full p-3 bg-gray-700 border-2 border-red-500 rounded-lg focus:ring-2 focus:ring-red-400 text-gray-100 placeholder-gray-400"
                                } else {
                                    "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100 placeholder-gray-400"
                                },
                                placeholder: "Choose a username (min. 3 characters)",
                                value: "{username}",
                                oninput: move |evt| {
                                    username.set(evt.value());
                                    if username_touched() {
                                        validate_username();
                                    }
                                    if form_error().is_some() {
                                        form_error.set(None);
                                    }
                                },
                                onblur: move |_| {
                                    username_touched.set(true);
                                    validate_username();
                                },
                                minlength: 3,
                                required: true
                            }
                            if let Some(err) = username_error() {
                                div { class: "mt-1 text-sm text-red-400", "{err}" }
                            }
                        }

                        div { class: "mb-4",
                            label { class: "block text-gray-300 font-medium mb-2", "Email" }
                            input {
                                r#type: "email",
                                class: if email_error().is_some() {
                                    "w-full p-3 bg-gray-700 border-2 border-red-500 rounded-lg focus:ring-2 focus:ring-red-400 text-gray-100 placeholder-gray-400"
                                } else {
                                    "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100 placeholder-gray-400"
                                },
                                placeholder: "Enter your email address",
                                value: "{email}",
                                oninput: move |evt| {
                                    email.set(evt.value());
                                    if email_touched() {
                                        validate_email_fn();
                                    }
                                    if form_error().is_some() {
                                        form_error.set(None);
                                    }
                                },
                                onblur: move |_| {
                                    email_touched.set(true);
                                    validate_email_fn();
                                },
                                required: true
                            }
                            if let Some(err) = email_error() {
                                div { class: "mt-1 text-sm text-red-400", "{err}" }
                            }
                        }

                        div { class: "mb-6",
                            label { class: "block text-gray-300 font-medium mb-2", "Password" }
                            input {
                                r#type: "password",
                                class: if password_error().is_some() {
                                    "w-full p-3 bg-gray-700 border-2 border-red-500 rounded-lg focus:ring-2 focus:ring-red-400 text-gray-100 placeholder-gray-400"
                                } else {
                                    "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100 placeholder-gray-400"
                                },
                                placeholder: "Create a password (min. 6 characters)",
                                value: "{password}",
                                oninput: move |evt| {
                                    password.set(evt.value());
                                    if password_touched() {
                                        validate_password();
                                    }
                                    if form_error().is_some() {
                                        form_error.set(None);
                                    }
                                },
                                onblur: move |_| {
                                    password_touched.set(true);
                                    validate_password();
                                },
                                minlength: 6,
                                required: true
                            }
                            if let Some(err) = password_error() {
                                div { class: "mt-1 text-sm text-red-400", "{err}" }
                            }
                        }

                        if let Some(err) = form_error() {
                            div { class: "mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg animate-shake",
                                div { class: "flex items-center space-x-3",
                                    svg { class: "w-5 h-5 text-red-400 flex-shrink-0", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                    div { class: "text-sm text-red-300", "{err}" }
                                }
                            }
                        }

                        div { class: "flex gap-3",
                            button {
                                r#type: "submit",
                                class: "flex-1 bg-gradient-to-r from-green-500 to-emerald-600 text-white p-3 rounded-lg font-medium hover:from-green-600 hover:to-emerald-700 transition-all",
                                "Register"
                            }
                            button {
                                r#type: "button",
                                class: "flex-1 bg-gray-600 text-gray-200 p-3 rounded-lg font-medium hover:bg-gray-500 transition-all",
                                onclick: move |_| {
                                    show.set(false);
                                    username.set(String::new());
                                    email.set(String::new());
                                    password.set(String::new());
                                    username_error.set(None);
                                    email_error.set(None);
                                    password_error.set(None);
                                    form_error.set(None);
                                    username_touched.set(false);
                                    email_touched.set(false);
                                    password_touched.set(false);
                                },
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
    }
}
