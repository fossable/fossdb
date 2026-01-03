use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Success,
}

#[component]
pub fn Button(
    variant: ButtonVariant,
    onclick: EventHandler<MouseEvent>,
    children: Element,
    #[props(default = false)] disabled: bool,
) -> Element {
    let class_name = match variant {
        ButtonVariant::Primary => "px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 text-white rounded-lg font-medium hover:from-blue-600 hover:to-purple-700 transition-all shadow-lg hover:shadow-xl",
        ButtonVariant::Secondary => "px-4 py-2 text-blue-400 hover:bg-gray-700 rounded-lg font-medium transition-all",
        ButtonVariant::Danger => "px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg font-medium transition-all",
        ButtonVariant::Success => "px-4 py-2 bg-green-500 hover:bg-green-600 text-white rounded-lg font-medium transition-all",
    };

    rsx! {
        button {
            class: "{class_name}",
            disabled: disabled,
            onclick: move |evt| onclick.call(evt),
            {children}
        }
    }
}
