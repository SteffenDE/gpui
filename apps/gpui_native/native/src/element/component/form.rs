#[cfg(not(feature = "components"))]
use super::render_component_fallback;
#[cfg(feature = "components")]
use super::{apply_component_styles, component_input_height, constrain_full_size_component};
#[cfg(feature = "components")]
use crate::element::component_registry::{
    filter_options, ComponentCombobox, ComponentInput, ComponentSelect, NativeComboboxDelegate,
    NativeSelectOption, SharedEvent, SharedQuery,
};
#[cfg(feature = "components")]
use crate::element::controlled::{ControlledBinding, SharedBinding};
#[cfg(not(feature = "components"))]
use crate::InputNode;
#[cfg(feature = "components")]
use crate::SelectOptionNode;
use crate::{
    gpui, ComboboxComponentNode, ElementRenderContext, InputComponentNode, SelectComponentNode,
};
#[cfg(feature = "components")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "components")]
#[derive(Debug, PartialEq)]
struct InputAccessibility {
    label: String,
    role: gpui::Role,
    value: Option<String>,
    placeholder: Option<String>,
}

#[cfg(feature = "components")]
fn input_accessibility(
    label: String,
    value: String,
    placeholder: Option<String>,
    masked: bool,
    multiline: bool,
) -> InputAccessibility {
    InputAccessibility {
        label,
        role: if masked {
            gpui::Role::PasswordInput
        } else if multiline {
            gpui::Role::MultilineTextInput
        } else {
            gpui::Role::TextInput
        },
        value: (!masked).then_some(value),
        placeholder,
    }
}

#[cfg(feature = "components")]
#[derive(Debug, PartialEq)]
struct ChoiceAccessibility {
    label: String,
    value: Option<String>,
    placeholder: Option<String>,
}

#[cfg(feature = "components")]
fn choice_accessibility(
    label: String,
    value: Option<&str>,
    placeholder: Option<String>,
    options: &[SelectOptionNode],
) -> ChoiceAccessibility {
    ChoiceAccessibility {
        label,
        value: selected_choice_label(value, options),
        placeholder,
    }
}

#[cfg(feature = "components")]
fn selected_choice_label(value: Option<&str>, options: &[SelectOptionNode]) -> Option<String> {
    value.and_then(|value| {
        options
            .iter()
            .find(|option| option.value == value)
            .map(|option| option.label.clone())
    })
}

#[cfg(feature = "components")]
fn prepare_insert_text(value: &str, start: usize, text: &str, on_new_line: bool) -> String {
    let before = value.get(..start).unwrap_or_default();
    if on_new_line && !before.is_empty() && !before.ends_with('\n') {
        format!("\n{text}")
    } else {
        text.to_string()
    }
}

#[cfg(feature = "components")]
pub(crate) fn render_input_component(
    _element_id: usize,
    node: InputComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use crate::{push_event, EventValue, InputKind, NativeEvent};
    use gpui::{
        AppContext, Focusable, InteractiveElement, IntoElement, ParentElement,
        StatefulInteractiveElement, Styled,
    };
    use gpui_component::{
        input::{Input, InputEvent, InputState},
        Sizable,
    };

    let component_height = component_input_height(node.size.as_deref(), node.style.height);
    let accessibility = input_accessibility(
        node.label.clone(),
        node.value.clone(),
        node.placeholder.clone(),
        node.masked,
        node.multiline,
    );

    if context.components.input_mut(&node.id).is_none() {
        let state = context.cx.new(|cx| {
            let mut state = InputState::new(context.window, cx)
                .default_value(node.value.clone())
                .placeholder(node.placeholder.clone().unwrap_or_default());
            if node.multiline {
                state = state
                    .auto_grow(1, node.max_rows.clamp(1, 64) as usize)
                    .submit_on_enter(node.submit_on_enter);
            }
            if node.masked {
                state = state.masked(true);
            }
            state
        });
        let binding: SharedBinding<String> = Arc::new(Mutex::new(ControlledBinding::new(
            node.change.clone(),
            node.value.clone(),
        )));
        let submit_event: SharedEvent = Arc::new(Mutex::new(node.submit.clone()));
        let runtime = context.runtime.clone();
        let runtime_for_subscription = runtime.clone();
        let window_id = context.window_id;
        let event_binding = binding.clone();
        let event_submit = submit_event.clone();
        let suppressed_changes = Arc::new(Mutex::new(0usize));
        let event_suppressed_changes = suppressed_changes.clone();
        let multiline = node.multiline;
        let submit_on_enter = node.submit_on_enter;
        let subscription = context.cx.subscribe_in(
            &state,
            context.window,
            move |_root, state, event: &InputEvent, _window, cx| {
                let value = state.read(cx).value().to_string();

                // InputState emits Change for inserted newlines; PressEnter is
                // an additional notification, not another text edit.
                match (event, multiline, submit_on_enter) {
                    (InputEvent::Change, _, _) => {
                        let suppressed = event_suppressed_changes
                            .lock()
                            .map(|mut count| {
                                if *count > 0 {
                                    *count -= 1;
                                    true
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);
                        if suppressed {
                            return;
                        }
                        let event = event_binding.lock().ok().and_then(|mut binding| {
                            binding.event.clone().inspect(|_event| {
                                binding.push_pending(value.clone());
                            })
                        });

                        if let Some(event) = event {
                            let result = push_event(
                                &runtime_for_subscription,
                                NativeEvent::Input {
                                    kind: InputKind::Change,
                                    window_id,
                                    event,
                                    value: Some(EventValue::String(value)),
                                },
                            );
                            if result.is_err() {
                                if let Ok(mut binding) = event_binding.lock() {
                                    binding.pop_pending();
                                }
                            }
                        }
                    }
                    (InputEvent::PressEnter { shift: true, .. }, true, _)
                    | (
                        InputEvent::PressEnter {
                            secondary: false, ..
                        },
                        true,
                        false,
                    ) => {}
                    (InputEvent::PressEnter { .. }, _, _) => {
                        let event = event_submit.lock().ok().and_then(|event| event.clone());
                        if let Some(event) = event {
                            let _ = push_event(
                                &runtime_for_subscription,
                                NativeEvent::Input {
                                    kind: InputKind::Submit,
                                    window_id,
                                    event,
                                    value: Some(EventValue::String(value)),
                                },
                            );
                        }
                    }
                    (InputEvent::Focus | InputEvent::Blur, _, _) => {}
                }
            },
        );

        let input_id = node.id.clone();
        let focus_event = node.focus.clone();
        let blur_event = node.blur.clone();
        let runtime_for_focus = runtime.clone();
        let focus_handle = state.focus_handle(context.cx);
        if let Ok(mut bindings) = runtime_for_focus.focus_bindings.lock() {
            bindings.insert((window_id, input_id.clone()), (focus_event, blur_event));
        }
        // A component removed from the tree loses its InputState. Reusing its
        // logical id creates a new FocusHandle, so bind observers to that new
        // handle even if the previous instance used the same id.
        if let Ok(mut observers) = runtime_for_focus.focus_observers.lock() {
            observers.insert((window_id, input_id.clone()));
        }
        crate::element::install_focus_observers(
            focus_handle,
            input_id,
            runtime_for_focus,
            window_id,
            context.window,
            context.cx,
        );
        context.components.insert_input(
            &node.id,
            ComponentInput {
                state,
                binding,
                submit_event,
                focus_request: 0,
                insert_request: 0,
                suppressed_changes,
                placeholder: node.placeholder.clone().unwrap_or_default(),
                masked: node.masked,
                loading: false,
                _subscription: subscription,
            },
        );
    }

    let input = context
        .components
        .input_mut(&node.id)
        .expect("component input should exist");
    let apply_value = input
        .binding
        .lock()
        .map(|mut binding| {
            binding.event = node.change.clone();
            binding.reconcile(&node.value)
        })
        .unwrap_or(true);
    if let Ok(mut submit_event) = input.submit_event.lock() {
        *submit_event = node.submit.clone();
    }
    if let Ok(mut bindings) = context.runtime.focus_bindings.lock() {
        bindings.insert(
            (context.window_id, node.id.clone()),
            (node.focus.clone(), node.blur.clone()),
        );
    }
    let request_focus = input.focus_request != node.focus_request;
    input.focus_request = node.focus_request;
    let current_value = input.state.read(context.cx).value();
    if apply_value && current_value.as_ref() != node.value {
        input.state.update(context.cx, |state, cx| {
            state.set_value(node.value.clone(), context.window, cx)
        });
    }

    if input.insert_request != node.insert_request {
        input.insert_request = node.insert_request;
        if node.insert_request > 0 {
            let before = input.state.read(context.cx).value().to_string();
            let value = if node.disabled {
                before.clone()
            } else {
                input.state.update(context.cx, |state, cx| {
                    let selected = state.selected_range();
                    let found = node.insert_match.as_deref().and_then(|needle| {
                        if needle.is_empty() {
                            None
                        } else {
                            before.find(needle).map(|start| start..start + needle.len())
                        }
                    });
                    if node.insert_match.is_none() || found.is_some() {
                        let range = found.clone().unwrap_or_else(|| selected.clone());
                        let text = prepare_insert_text(
                            &before,
                            range.start,
                            node.insert_text.as_deref().unwrap_or_default(),
                            node.insert_on_new_line,
                        );
                        if found.is_some() {
                            state.set_selected_range(range.clone(), cx);
                        }
                        if before.get(range.clone()) != Some(text.as_str()) {
                            if let Ok(mut count) = input.suppressed_changes.lock() {
                                *count += 1;
                            }
                            state.replace(text.clone(), context.window, cx);
                        } else if found.is_none() {
                            state.set_selected_range(range.end..range.end, cx);
                        }
                        if found.is_some() {
                            let adjust = |offset: usize| {
                                if offset < range.start {
                                    offset
                                } else if offset >= range.end {
                                    offset - range.len() + text.len()
                                } else {
                                    range.start + text.len()
                                }
                            };
                            state.set_selected_range(
                                adjust(selected.start)..adjust(selected.end),
                                cx,
                            );
                        }
                    }
                    state.value().to_string()
                })
            };
            if value != before {
                let event = input.binding.lock().ok().and_then(|mut binding| {
                    binding
                        .event
                        .clone()
                        .inspect(|_| binding.push_pending(value.clone()))
                });
                if let Some(event) = event {
                    let result = push_event(
                        &context.runtime,
                        NativeEvent::Input {
                            kind: InputKind::Change,
                            window_id: context.window_id,
                            event,
                            value: Some(EventValue::String(value.clone())),
                        },
                    );
                    if result.is_err() {
                        if let Ok(mut binding) = input.binding.lock() {
                            binding.pop_pending();
                        }
                    }
                }
            }
            if let Some(event) = node.inserted.clone() {
                let _ = push_event(
                    &context.runtime,
                    NativeEvent::Input {
                        kind: InputKind::Change,
                        window_id: context.window_id,
                        event,
                        value: Some(EventValue::Strings(vec![
                            node.insert_request.to_string(),
                            value,
                        ])),
                    },
                );
            }
        }
    }

    let placeholder = node.placeholder.unwrap_or_default();
    if input.placeholder != placeholder {
        input.placeholder = placeholder.clone();
        input.state.update(context.cx, |state, cx| {
            state.set_placeholder(placeholder, context.window, cx)
        });
    }
    if input.masked != node.masked {
        input.masked = node.masked;
        input.state.update(context.cx, |state, cx| {
            state.set_masked(node.masked, context.window, cx)
        });
    }
    if input.loading != node.loading {
        input.loading = node.loading;
        input.state.update(context.cx, |state, cx| {
            state.set_loading(node.loading, context.window, cx)
        });
    }

    let accessibility_value = accessibility
        .value
        .as_ref()
        .map(|_value| input.state.read(context.cx).value().to_string());
    let focus_handle = input.state.focus_handle(context.cx);
    if request_focus && node.focus_request > 0 {
        crate::element::focus_after_frame(focus_handle.clone(), context.window, context.cx);
    }
    let mut element = Input::new(&input.state)
        .role(gpui::Role::GenericContainer)
        .appearance(node.appearance)
        .disabled(node.disabled)
        .cleanable(node.cleanable);
    element = match node.size.as_deref() {
        Some("xs") => element.xsmall(),
        Some("sm") => element.small(),
        Some("lg") => element.large(),
        _ => element,
    };
    if node.masked {
        element = element.mask_toggle();
    }

    let mut accessible_element = gpui::div()
        .id(format!("input-accessibility-{}", node.id))
        .role(accessibility.role)
        .aria_label(accessibility.label)
        .track_focus(&focus_handle.tab_stop(!node.disabled))
        .on_key_down(|event, _window, cx| {
            if event.keystroke.key == "enter" {
                cx.stop_propagation();
            }
        })
        .flex()
        .w_full();
    if node.multiline && !node.submit_on_enter {
        let state = input.state.clone();
        let submit_event = input.submit_event.clone();
        let runtime = context.runtime.clone();
        let window_id = context.window_id;
        accessible_element = accessible_element.capture_action(
            move |action: &gpui_component::input::Enter, window, cx| {
                if action.secondary && !action.shift {
                    if let Some(event) = submit_event.lock().ok().and_then(|event| event.clone()) {
                        let _ = push_event(
                            &runtime,
                            NativeEvent::Input {
                                kind: InputKind::Submit,
                                window_id,
                                event,
                                value: Some(EventValue::String(state.read(cx).value().to_string())),
                            },
                        );
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                }
            },
        );
    }
    if let Some(event) = node.keystroke.clone() {
        let runtime = context.runtime.clone();
        let window_id = context.window_id;
        let keys = node.capture_keys.clone();
        accessible_element = accessible_element.capture_key_down(move |key, window, cx| {
            crate::element::event::emit_structured_key(
                &runtime,
                window_id,
                &event,
                &key.keystroke,
                key.is_held,
            );
            let m = key.keystroke.modifiers;
            if !m.control && !m.platform && !m.alt && !m.shift && keys.contains(&key.keystroke.key)
            {
                window.prevent_default();
                cx.stop_propagation();
            }
        });
    }
    if node.multiline {
        accessible_element = accessible_element.h_auto();
    } else {
        accessible_element = accessible_element.h(gpui::px(component_height));
    }

    // A one-line input ignores Up and Down, so they are reported instead: a
    // search box can move a highlight through its results. They are captured
    // on the way down, before the input's own no-op handlers stop them.
    if let Some(event) = node.keydown.clone().filter(|_| !node.multiline) {
        let runtime = context.runtime.clone();
        let window_id = context.window_id;

        let report = move |key: &'static str| {
            let _ = push_event(
                &runtime,
                NativeEvent::Input {
                    kind: InputKind::KeyDown,
                    window_id,
                    event: event.clone(),
                    value: Some(EventValue::String(key.to_string())),
                },
            );
        };

        let report_down = report.clone();
        accessible_element = accessible_element
            .capture_action(move |_: &gpui_component::input::MoveUp, _window, _cx| report("up"))
            .capture_action(move |_: &gpui_component::input::MoveDown, _window, _cx| {
                report_down("down")
            });
    }
    if let Some(value) = accessibility_value {
        accessible_element = accessible_element.aria_value(value);
    }
    if let Some(placeholder) = accessibility.placeholder {
        accessible_element = accessible_element.aria_placeholder(placeholder);
    }

    accessible_element
        .child(apply_component_styles(element, node.style))
        .into_any_element()
}

#[cfg(feature = "components")]
pub(crate) fn render_select_component(
    node: SelectComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use crate::{push_event, EventValue, InputKind, NativeEvent};
    use gpui::{
        AppContext, Focusable, InteractiveElement, IntoElement, ParentElement,
        StatefulInteractiveElement, Styled,
    };

    let component_height = component_input_height(node.size.as_deref(), node.style.height);
    let mut accessibility = choice_accessibility(
        node.label.clone(),
        node.value.as_deref(),
        node.placeholder.clone(),
        &node.options,
    );
    use gpui_component::{
        select::{Select, SelectEvent, SelectState},
        IndexPath, Sizable,
    };

    let options = native_select_options(&node.options);

    if context.components.select_mut(&node.id).is_none() {
        let selected_index = node.value.as_ref().and_then(|value| {
            options
                .iter()
                .position(|option| option.value.as_ref() == value)
                .map(IndexPath::new)
        });
        let state = context
            .cx
            .new(|cx| SelectState::new(options.clone(), selected_index, context.window, cx));
        let binding: SharedBinding<Option<String>> = Arc::new(Mutex::new(ControlledBinding::new(
            node.change.clone(),
            node.value.clone(),
        )));
        let runtime = context.runtime.clone();
        let window_id = context.window_id;
        let event_binding = binding.clone();
        let subscription = context.cx.subscribe_in(
            &state,
            context.window,
            move |_root, _state, event: &SelectEvent<Vec<NativeSelectOption>>, _window, _cx| {
                let SelectEvent::Confirm(value) = event;
                let value = value.as_ref().map(ToString::to_string);
                let event = event_binding.lock().ok().and_then(|mut binding| {
                    binding.event.clone().inspect(|_event| {
                        binding.push_pending(value.clone());
                    })
                });

                if let Some(event) = event {
                    let event_value = value
                        .clone()
                        .map(EventValue::String)
                        .unwrap_or(EventValue::Nil);
                    let result = push_event(
                        &runtime,
                        NativeEvent::Input {
                            kind: InputKind::Change,
                            window_id,
                            event,
                            value: Some(event_value),
                        },
                    );
                    if result.is_err() {
                        if let Ok(mut binding) = event_binding.lock() {
                            binding.pop_pending();
                        }
                    }
                }
            },
        );

        context.components.insert_select(
            &node.id,
            ComponentSelect {
                state,
                binding,
                options: options.clone(),
                _subscription: subscription,
            },
        );
    }

    let select = context
        .components
        .select_mut(&node.id)
        .expect("component select should exist");
    let options_changed = select.options != options;
    if options_changed {
        select.options = options.clone();
        select.state.update(context.cx, |state, cx| {
            state.set_items(options, context.window, cx)
        });
    }

    let apply_value = select
        .binding
        .lock()
        .map(|mut binding| {
            binding.event = node.change.clone();
            binding.reconcile(&node.value)
        })
        .unwrap_or(true)
        || options_changed;
    let current_value = select
        .state
        .read(context.cx)
        .selected_value()
        .map(ToString::to_string);
    if apply_value && (options_changed || current_value != node.value) {
        let value = node.value.clone();
        select.state.update(context.cx, |state, cx| {
            match value {
                Some(value) => state.set_selected_value(&value.into(), context.window, cx),
                None => state.set_selected_index(None, context.window, cx),
            }
            cx.notify();
        });
    }

    let current_accessibility_value = select
        .state
        .read(context.cx)
        .selected_value()
        .map(ToString::to_string);
    accessibility.value =
        selected_choice_label(current_accessibility_value.as_deref(), &node.options);
    let focus_handle = select.state.focus_handle(context.cx);
    let mut element = Select::new(&select.state)
        .disabled(node.disabled)
        .cleanable(node.cleanable);
    if let Some(placeholder) = node.placeholder {
        element = element.placeholder(placeholder);
    }
    element = match node.size.as_deref() {
        Some("xs") => element.xsmall(),
        Some("sm") => element.small(),
        Some("lg") => element.large(),
        _ => element,
    };

    let mut accessible_element = gpui::div()
        .id(format!("select-accessibility-{}", node.id))
        .role(gpui::Role::ComboBox)
        .aria_label(accessibility.label)
        .track_focus(&focus_handle.tab_stop(!node.disabled))
        .flex()
        .h(gpui::px(component_height));
    if let Some(value) = accessibility.value {
        accessible_element = accessible_element.aria_value(value);
    }
    if let Some(placeholder) = accessibility.placeholder {
        accessible_element = accessible_element.aria_placeholder(placeholder);
    }

    accessible_element
        .child(apply_component_styles(element, node.style))
        .into_any_element()
}

#[cfg(feature = "components")]
pub(crate) fn render_combobox_component(
    node: ComboboxComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use crate::{push_event, EventValue, InputKind, NativeEvent};
    use gpui::{
        AppContext, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
        Styled,
    };

    let component_height = component_input_height(node.size.as_deref(), node.style.height);
    let mut accessibility = choice_accessibility(
        node.label.clone(),
        node.value.as_deref(),
        node.placeholder.clone(),
        &node.options,
    );
    use gpui_component::{
        combobox::{Combobox, ComboboxEvent, ComboboxState},
        IndexPath, Sizable,
    };

    let options = native_select_options(&node.options);

    if context.components.combobox_mut(&node.id).is_none() {
        let selected_indices = node
            .value
            .as_ref()
            .and_then(|value| {
                options
                    .iter()
                    .position(|option| option.value.as_ref() == value)
            })
            .map(IndexPath::new)
            .into_iter()
            .collect::<Vec<_>>();
        let search_event: SharedEvent = Arc::new(Mutex::new(node.search.clone()));
        let query: SharedQuery = Arc::new(Mutex::new(String::new()));
        let delegate = NativeComboboxDelegate::new(
            options.clone(),
            context.runtime.clone(),
            context.window_id,
            search_event.clone(),
            query.clone(),
        );
        let state = context.cx.new(|cx| {
            ComboboxState::new(delegate, selected_indices, context.window, cx).searchable(true)
        });
        let binding: SharedBinding<Option<String>> = Arc::new(Mutex::new(ControlledBinding::new(
            node.change.clone(),
            node.value.clone(),
        )));
        let runtime = context.runtime.clone();
        let window_id = context.window_id;
        let event_binding = binding.clone();
        let subscription = context.cx.subscribe_in(
            &state,
            context.window,
            move |_root, _state, event: &ComboboxEvent<NativeComboboxDelegate>, _window, _cx| {
                let ComboboxEvent::Change(values) = event else {
                    return;
                };
                let value = values.first().map(ToString::to_string);
                let event = event_binding.lock().ok().and_then(|mut binding| {
                    binding.event.clone().inspect(|_event| {
                        binding.push_pending(value.clone());
                    })
                });

                if let Some(event) = event {
                    let event_value = value
                        .clone()
                        .map(EventValue::String)
                        .unwrap_or(EventValue::Nil);
                    let result = push_event(
                        &runtime,
                        NativeEvent::Input {
                            kind: InputKind::Change,
                            window_id,
                            event,
                            value: Some(event_value),
                        },
                    );
                    if result.is_err() {
                        if let Ok(mut binding) = event_binding.lock() {
                            binding.pop_pending();
                        }
                    }
                }
            },
        );

        context.components.insert_combobox(
            &node.id,
            ComponentCombobox {
                state,
                binding,
                search_event,
                query,
                options: options.clone(),
                _subscription: subscription,
            },
        );
    }

    let combobox = context
        .components
        .combobox_mut(&node.id)
        .expect("component combobox should exist");
    if let Ok(mut search_event) = combobox.search_event.lock() {
        *search_event = node.search.clone();
    }

    let options_changed = combobox.options != options;
    if options_changed {
        combobox.options = options.clone();
        let delegate = NativeComboboxDelegate::new(
            options.clone(),
            context.runtime.clone(),
            context.window_id,
            combobox.search_event.clone(),
            combobox.query.clone(),
        );
        combobox.state.update(context.cx, |state, cx| {
            state.set_items(delegate, context.window, cx)
        });
    }

    let apply_value = combobox
        .binding
        .lock()
        .map(|mut binding| {
            binding.event = node.change.clone();
            binding.reconcile(&node.value)
        })
        .unwrap_or(true)
        || options_changed;
    let current_value = combobox
        .state
        .read(context.cx)
        .selected_value()
        .map(|value| value.to_string());
    if apply_value && current_value != node.value {
        let query = combobox
            .query
            .lock()
            .map(|query| query.clone())
            .unwrap_or_default();
        let filtered = filter_options(&options, &query);
        let selected_indices = node
            .value
            .as_ref()
            .and_then(|value| {
                filtered
                    .iter()
                    .position(|option| option.value.as_ref() == value)
            })
            .map(IndexPath::new)
            .into_iter()
            .collect::<Vec<_>>();
        combobox.state.update(context.cx, |state, cx| {
            state.set_selected_indices(selected_indices, context.window, cx)
        });
    }

    let current_accessibility_value = combobox
        .state
        .read(context.cx)
        .selected_value()
        .map(|value| value.to_string());
    accessibility.value =
        selected_choice_label(current_accessibility_value.as_deref(), &node.options);
    let mut element = Combobox::new(&combobox.state)
        .disabled(node.disabled || node.loading)
        .cleanable(node.cleanable)
        // The pinned upstream locale uses ComboBox, but its default lookup uses Combobox.
        .placeholder("Please select")
        .search_placeholder("Search...");
    if let Some(placeholder) = node.placeholder {
        element = element.placeholder(placeholder);
    }
    if let Some(search_placeholder) = node.search_placeholder {
        element = element.search_placeholder(search_placeholder);
    }
    element = match node.size.as_deref() {
        Some("xs") => element.xsmall(),
        Some("sm") => element.small(),
        Some("lg") => element.large(),
        _ => element,
    };

    let mut accessible_element = gpui::div()
        .id(format!("combobox-accessibility-{}", node.id))
        .role(gpui::Role::Group)
        .aria_label(accessibility.label)
        .w_full();
    if let Some(value) = accessibility.value {
        accessible_element = accessible_element.aria_value(value);
    }
    if let Some(placeholder) = accessibility.placeholder {
        accessible_element = accessible_element.aria_placeholder(placeholder);
    }

    accessible_element
        .child(constrain_full_size_component(
            apply_component_styles(element, node.style),
            component_height,
        ))
        .into_any_element()
}

#[cfg(not(feature = "components"))]
pub(crate) fn render_combobox_component(
    node: ComboboxComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    let label = node
        .value
        .as_ref()
        .and_then(|value| node.options.iter().find(|option| &option.value == value))
        .map(|option| option.label.clone())
        .or(node.placeholder);

    render_component_fallback(node.style, label, Vec::new(), context)
}

#[cfg(not(feature = "components"))]
pub(crate) fn render_select_component(
    node: SelectComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    let label = node
        .value
        .as_ref()
        .and_then(|value| node.options.iter().find(|option| &option.value == value))
        .map(|option| option.label.clone())
        .or(node.placeholder);

    render_component_fallback(node.style, label, Vec::new(), context)
}

#[cfg(not(feature = "components"))]
pub(crate) fn render_input_component(
    element_id: usize,
    node: InputComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    super::super::render_input_primitive(
        element_id,
        InputNode {
            style: node.style,
            id: Some(node.id),
            value: node.value,
            placeholder: node.placeholder,
            capture_keys: Vec::new(),
            select_all_request: 0,
            focus_request: node.focus_request,
            change: node.change,
            keydown: node.keydown,
            keyup: None,
            keystroke: None,
            keyrelease: None,
            focus: node.focus,
            blur: node.blur,
        },
        context,
    )
}

#[cfg(all(test, feature = "components"))]
mod tests {
    use super::{
        choice_accessibility, input_accessibility, ChoiceAccessibility, InputAccessibility,
    };
    use crate::SelectOptionNode;

    #[test]
    fn input_accessibility_hides_masked_values() {
        assert_eq!(
            input_accessibility(
                "Display name".to_string(),
                "Ada".to_string(),
                Some("Name".to_string()),
                false,
                false,
            ),
            InputAccessibility {
                label: "Display name".to_string(),
                role: crate::gpui::Role::TextInput,
                value: Some("Ada".to_string()),
                placeholder: Some("Name".to_string()),
            }
        );

        assert_eq!(
            input_accessibility(
                "Password".to_string(),
                "secret".to_string(),
                None,
                true,
                true
            ),
            InputAccessibility {
                label: "Password".to_string(),
                role: crate::gpui::Role::PasswordInput,
                value: None,
                placeholder: None,
            }
        );
    }

    #[test]
    fn multiline_input_accessibility_exposes_its_role_and_value() {
        let accessibility = input_accessibility(
            "Message".to_string(),
            "First\nSecond".to_string(),
            None,
            false,
            true,
        );
        assert_eq!(accessibility.role, crate::gpui::Role::MultilineTextInput);
        assert_eq!(accessibility.value.as_deref(), Some("First\nSecond"));
    }

    #[test]
    fn choice_accessibility_uses_the_selected_visible_label() {
        let options = vec![SelectOptionNode {
            label: "Elixir".to_string(),
            value: "ex".to_string(),
        }];

        assert_eq!(
            choice_accessibility(
                "Language".to_string(),
                Some("ex"),
                Some("Choose a language".to_string()),
                &options,
            ),
            ChoiceAccessibility {
                label: "Language".to_string(),
                value: Some("Elixir".to_string()),
                placeholder: Some("Choose a language".to_string()),
            }
        );
    }
}

#[cfg(feature = "components")]
fn native_select_options(options: &[SelectOptionNode]) -> Vec<NativeSelectOption> {
    options
        .iter()
        .map(|option| NativeSelectOption {
            label: option.label.clone().into(),
            value: option.value.clone().into(),
        })
        .collect()
}
