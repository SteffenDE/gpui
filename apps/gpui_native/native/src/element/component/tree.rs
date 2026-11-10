use crate::element::ElementRenderContext;
use crate::{gpui, TreeComponentNode, TreeItemComponentNode};

#[cfg(not(feature = "components"))]
use super::render_component_fallback;
#[cfg(feature = "components")]
use crate::element::ElementNode;

#[cfg(feature = "components")]
use std::collections::HashSet;
#[cfg(feature = "components")]
use std::ops::Range;
#[cfg(feature = "components")]
use std::sync::Arc;

#[cfg(feature = "components")]
pub(crate) type ComponentTree = super::uniform_collection::ComponentUniformCollection;

#[cfg(feature = "components")]
#[derive(Clone, Debug)]
struct TreeItem {
    id: String,
    parent_id: Option<String>,
    level: usize,
    branch: bool,
    expanded: bool,
    position: Option<usize>,
    set_size: Option<usize>,
    disabled: bool,
    style: crate::StyleAttrs,
    children: Vec<ElementNode>,
}

#[cfg(feature = "components")]
#[derive(Clone)]
struct VisibleTree {
    id: String,
    items: Arc<Vec<TreeItem>>,
    offset: usize,
    total_count: usize,
    overscan: usize,
    selected: Option<String>,
    multiple: bool,
    selection_items: Arc<Vec<String>>,
    selection_event: Option<String>,
    navigation_event: Option<String>,
    selected_background: gpui::Hsla,
    change_event: String,
    open_event: Option<String>,
    toggle_event: String,
    range_event: Option<String>,
    item_height: f32,
    toggle_on_click: bool,
    select_on_navigation: bool,
    focus_color: u32,
}

#[cfg(feature = "components")]
#[derive(Clone)]
struct TreeKey {
    id: String,
    parent_id: Option<String>,
    level: usize,
    branch: bool,
    expanded: bool,
    disabled: bool,
    index: usize,
}

#[cfg(feature = "components")]
pub(crate) fn render(
    node: TreeComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use crate::element::apply_generated_render_styles;
    use gpui::{
        uniform_list, InteractiveElement, IntoElement, ParentElement, Role,
        StatefulInteractiveElement, Styled,
    };

    use gpui_component::ActiveTheme;

    let items = node
        .children
        .into_iter()
        .filter_map(|child| match child {
            ElementNode::TreeItemComponent(item) => Some(TreeItem {
                id: item.id,
                parent_id: item.parent_id,
                level: usize::try_from(item.level).unwrap_or(usize::MAX).max(1),
                branch: item.branch,
                expanded: item.expanded,
                position: item.position.and_then(|value| usize::try_from(value).ok()),
                set_size: item.set_size.and_then(|value| usize::try_from(value).ok()),
                disabled: item.disabled,
                style: item.style,
                children: item.children,
            }),
            _other => None,
        })
        .collect::<Vec<_>>();
    let items = Arc::new(items);
    let total_count = usize::try_from(node.total_count).unwrap_or(usize::MAX);
    let offset = usize::try_from(node.offset)
        .unwrap_or(usize::MAX)
        .min(total_count);
    let overscan = usize::try_from(node.overscan).unwrap_or(usize::MAX);
    let keys = items
        .iter()
        .enumerate()
        .map(|(local_index, item)| TreeKey {
            id: item.id.clone(),
            parent_id: item.parent_id.clone(),
            level: item.level,
            branch: item.branch,
            expanded: item.expanded,
            disabled: item.disabled,
            index: offset.saturating_add(local_index),
        })
        .collect::<Vec<_>>();
    let selected_index = super::uniform_collection::controlled_index(
        total_count,
        node.selected_index,
        node.selected.as_deref(),
        |selected| {
            keys.iter()
                .find(|key| key.id == selected)
                .map(|key| key.index)
        },
    );

    if context.components.tree_mut(&node.id).is_none() {
        let component = ComponentTree::new(context.cx);
        context.components.insert_tree(&node.id, component);
    }

    let component = context
        .components
        .tree_mut(&node.id)
        .expect("tree component must exist after insertion");
    let reveal = super::uniform_collection::controlled_reveal(
        total_count,
        node.reveal_index,
        node.reveal.as_deref(),
        |reveal| {
            keys.iter()
                .find(|key| key.id == reveal)
                .map(|key| key.index)
        },
    );
    component.reconcile_reveal(reveal, node.reveal_strategy.as_deref());
    component.reset_range_without_event(node.range.as_deref());
    let selection_items = Arc::new(if node.selection_items.is_empty() {
        keys.iter()
            .filter(|item| !item.disabled)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>()
    } else {
        node.selection_items.clone()
    });
    let navigation = component.tree_navigation.clone();
    if let Ok(mut navigation) = navigation.lock() {
        if navigation.selected != node.selected {
            navigation.selected = node.selected.clone();
            if !node.multiple || node.selected.is_some() {
                navigation.focused = node.selected.clone();
                navigation.group.anchor = node.selected.clone();
            }
        }
        if node.multiple {
            navigation.group.ids = node.selected_items.clone();
        }
    }
    let scroll_handle = component.scroll_handle.clone();
    let focus_handle = component.focus_handle.clone();

    let visible_tree = VisibleTree {
        id: node.id.clone(),
        items: items.clone(),
        offset,
        total_count,
        overscan,
        selected: node.selected.clone(),
        multiple: node.multiple,
        selection_items: selection_items.clone(),
        selection_event: node.selection_change.clone(),
        navigation_event: node.navigation_change.clone(),
        selected_background: node
            .style
            .selected_background
            .map(|color| gpui::rgba(color).into())
            .unwrap_or_else(|| context.cx.theme().accent),
        change_event: node.change.clone().unwrap_or_default(),
        open_event: node.open.clone(),
        toggle_event: node.toggle.clone().unwrap_or_default(),
        range_event: node.range.clone(),
        item_height: (node.item_height as f32).max(1.0),
        toggle_on_click: node.toggle_on_click,
        select_on_navigation: node.select_on_navigation,
        focus_color: node.style.focus_outline_color.unwrap_or(0x656D78FF),
    };
    let processor = context.cx.processor(move |root, range, window, cx| {
        render_visible(root, &visible_tree, range, window, cx)
    });

    let runtime = context.runtime.clone();
    let window_id = context.window_id;
    let change_event = node.change.unwrap_or_default();
    let toggle_event = node.toggle.unwrap_or_default();
    let key_items = keys.clone();
    let key_scroll = scroll_handle.clone();
    let disabled = node.disabled;
    let select_on_navigation = node.select_on_navigation;
    let toggle_on_click = node.toggle_on_click;
    let multiple = node.multiple;
    let selection_event = node.selection_change.clone();
    let navigation_event = node.navigation_change.clone();
    let open_event = node.open.clone();
    let selected_for_keys = if select_on_navigation {
        selected_index.and_then(|index| keys.iter().position(|key| key.index == index))
    } else {
        navigation.lock().ok().and_then(|navigation| {
            keys.iter()
                .position(|key| Some(&key.id) == navigation.focused.as_ref())
        })
    };

    let container = apply_generated_render_styles(gpui::div(), node.style).id(node.id.clone());
    crate::element::register_test_target(
        container,
        node.id.clone(),
        Some(focus_handle.clone()),
        context,
    )
    .role(Role::Tree)
    .aria_label(node.label.unwrap_or_else(|| "Tree".to_string()))
    .track_focus(&focus_handle.tab_stop(!disabled))
    .on_key_down(move |event, window, cx| {
        if disabled {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if multiple && (modifiers.control || modifiers.platform) && (key == "a" || key == "space") {
            let ids = if let Ok(mut navigation) = navigation.lock() {
                let focused = navigation.focused.clone();
                if key == "a" {
                    navigation.group.all(&selection_items, focused);
                } else if let Some(focused) = focused {
                    navigation.group.toggle(&focused);
                }
                navigation.keyboard = true;
                navigation.group.ids.clone()
            } else {
                return;
            };
            emit_selection(&runtime, window_id, selection_event.as_deref(), &ids);
            if let Some(last) = ids.last() {
                super::uniform_collection::emit_change(
                    &runtime,
                    window_id,
                    Some(&change_event),
                    last,
                );
            }
            window.refresh();
            cx.stop_propagation();
            return;
        }
        if multiple && modifiers.platform && key == "enter" {
            if let Some(open) = open_event.as_deref() {
                if let Some(position) = selected_for_keys {
                    emit_open(
                        &runtime,
                        window_id,
                        open,
                        &key_items[position].id,
                        modifiers,
                    );
                }
                cx.stop_propagation();
                return;
            }
        }
        let key = if !select_on_navigation
            && key == "right"
            && selected_for_keys.is_some_and(|position| !key_items[position].branch)
        {
            "down"
        } else {
            key
        };
        let Some(action) = key_action(key, &key_items, selected_for_keys, total_count) else {
            return;
        };
        match action {
            TreeAction::Select(position) => {
                let item = &key_items[position];
                let moved = if let Ok(mut navigation) = navigation.lock() {
                    let moved = navigation.focused.as_ref() != Some(&item.id);
                    navigation.focused = Some(item.id.clone());
                    navigation.keyboard = true;
                    moved
                } else {
                    false
                };
                if moved {
                    super::uniform_collection::emit_change(
                        &runtime,
                        window_id,
                        navigation_event.as_deref(),
                        &item.id,
                    );
                }
                let activate = matches!(key, "enter" | "space");
                if multiple
                    && (select_on_navigation
                        || activate
                        || (modifiers.shift && matches!(key, "up" | "down")))
                {
                    let ids = if let Ok(mut navigation) = navigation.lock() {
                        if !activate && modifiers.shift {
                            if let Some(current) =
                                selected_for_keys.map(|position| &key_items[position].id)
                            {
                                navigation.group.step(current, &item.id);
                            }
                        } else if modifiers.shift {
                            navigation.group.range(
                                &item.id,
                                &selection_items,
                                modifiers.control || modifiers.platform,
                            );
                        } else if activate && (modifiers.control || modifiers.platform) {
                            navigation.group.toggle(&item.id);
                        } else {
                            navigation.group.single(&item.id);
                        }
                        navigation.group.ids.clone()
                    } else {
                        return;
                    };
                    emit_selection(&runtime, window_id, selection_event.as_deref(), &ids);
                    if let Some(last) = ids.last() {
                        super::uniform_collection::emit_change(
                            &runtime,
                            window_id,
                            Some(&change_event),
                            last,
                        );
                    }
                    if activate && ids.last() != Some(&item.id) {
                        super::uniform_collection::emit_change(
                            &runtime,
                            window_id,
                            Some(&change_event),
                            &item.id,
                        );
                    }
                    if !activate {
                        key_scroll.scroll_to_item(item.index, gpui::ScrollStrategy::Nearest);
                        window.refresh();
                        cx.stop_propagation();
                        return;
                    }
                }
                if (select_on_navigation || activate) && !multiple {
                    super::uniform_collection::emit_change(
                        &runtime,
                        window_id,
                        Some(change_event.as_str()),
                        &item.id,
                    );
                    if activate && item.branch && toggle_on_click {
                        super::uniform_collection::emit_change(
                            &runtime,
                            window_id,
                            Some(toggle_event.as_str()),
                            &item.id,
                        );
                    }
                }
                if multiple
                    && activate
                    && item.branch
                    && toggle_on_click
                    && !modifiers.shift
                    && !modifiers.control
                    && !modifiers.platform
                {
                    super::uniform_collection::emit_change(
                        &runtime,
                        window_id,
                        Some(&toggle_event),
                        &item.id,
                    );
                }
                key_scroll.scroll_to_item(item.index, gpui::ScrollStrategy::Nearest);
            }
            TreeAction::Toggle(position) => {
                if let Ok(mut navigation) = navigation.lock() {
                    navigation.keyboard = true;
                }
                super::uniform_collection::emit_change(
                    &runtime,
                    window_id,
                    Some(toggle_event.as_str()),
                    &key_items[position].id,
                );
            }
        }
        window.refresh();
        cx.stop_propagation();
    })
    .child(
        uniform_list(
            format!("gpui-elixir-tree-{window_id}-{}", node.id),
            total_count,
            processor,
        )
        .track_scroll(&scroll_handle)
        .size_full(),
    )
    .into_any_element()
}

#[cfg(feature = "components")]
fn render_visible(
    root: &mut crate::ElixirRoot,
    tree: &VisibleTree,
    range: Range<usize>,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<'_, crate::ElixirRoot>,
) -> Vec<gpui::AnyElement> {
    use crate::element::apply_generated_render_styles;
    use gpui::{
        InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    };

    let runtime = root.runtime.clone();
    let window_id = root.window_id;
    let tree_id = tree.id.as_str();
    let Some(component) = root.components.tree_mut(tree_id) else {
        return Vec::new();
    };
    let navigation = component.tree_navigation.clone();
    let requested_range = range.start.saturating_sub(tree.overscan)
        ..range
            .end
            .saturating_add(tree.overscan)
            .min(tree.total_count);
    super::uniform_collection::schedule_range(
        component,
        super::uniform_collection::CollectionKind::Tree,
        &tree.id,
        requested_range,
        tree.range_event.as_deref(),
        &runtime,
        window_id,
        window,
        cx,
    );

    component.components.begin_render();
    let mut active_input_ids = HashSet::new();
    let mut rendered = Vec::new();

    let selected_items: HashSet<String> = if tree.multiple {
        navigation
            .lock()
            .map(|state| state.group.ids.iter().cloned().collect())
            .unwrap_or_default()
    } else {
        HashSet::new()
    };
    for index in range {
        let item = index
            .checked_sub(tree.offset)
            .and_then(|local_index| tree.items.get(local_index))
            .cloned();
        let Some(item) = item else {
            rendered.push(gpui::div().h(gpui::px(tree.item_height)).into_any_element());
            continue;
        };
        let item_id = item.id.clone();
        let selected = if tree.multiple {
            selected_items.contains(&item.id)
        } else {
            tree.selected.as_deref() == Some(item.id.as_str())
        };
        let item_runtime = runtime.clone();
        let item_focus = component.focus_handle.clone();
        let select_event = tree.change_event.clone();
        let open_event = tree.open_event.clone();
        let toggle_event = tree.toggle_event.clone();
        let event_id = item.id.clone();
        let branch = item.branch;
        let item_element_id = format!("gpui-elixir-tree-item-{window_id}-{tree_id}-{item_id}");
        let children = {
            let mut item_context = ElementRenderContext {
                pointer_observers: root.pointer_observers.clone(),
                scrolls: root.scrolls.clone(),
                horizontal_groups: root.horizontal_groups.clone(),
                tooltip_focus_owner: None,
                runtime: runtime.clone(),
                window_id,
                next_element_id: 0,
                id_namespace: format!("tree-{tree_id}-{item_id}"),
                active_input_ids: &mut active_input_ids,
                input_entities: &mut component.input_entities,
                terminal_entities: &mut component.terminal_entities,
                components: &mut component.components,
                window,
                cx,
            };
            item.children
                .into_iter()
                .map(|child| child.render(&mut item_context))
                .collect::<Vec<_>>()
        };
        let mut element = apply_generated_render_styles(gpui::div(), item.style)
            .h(gpui::px(tree.item_height))
            .id(item_element_id)
            .role(Role::TreeItem)
            .aria_level(item.level)
            .aria_selected(selected)
            .children(children);
        if tree.multiple && selected {
            element = element.bg(tree.selected_background).rounded(gpui::px(5.));
        }
        if let Some(position) = item.position {
            element = element.aria_position_in_set(position);
        }
        if let Some(set_size) = item.set_size {
            element = element.aria_size_of_set(set_size);
        }
        if item.branch {
            element = element.aria_expanded(item.expanded);
        }
        let (focused, keyboard) = navigation
            .lock()
            .map(|navigation| {
                (
                    navigation.focused.as_ref() == Some(&item.id),
                    navigation.keyboard,
                )
            })
            .unwrap_or_default();
        if (tree.select_on_navigation && selected) || (!tree.select_on_navigation && focused) {
            element = element.aria_active_descendant();
        }
        if !tree.select_on_navigation && focused && keyboard && item_focus.is_focused(window) {
            element = element.relative().child(
                gpui::div()
                    .absolute()
                    .inset_0()
                    .rounded(gpui::px(5.0))
                    .border_1()
                    .border_color(gpui::rgba(tree.focus_color)),
            );
        }
        let item_navigation = navigation.clone();
        let toggle_on_click = tree.toggle_on_click;
        let multiple = tree.multiple;
        let selection_items = tree.selection_items.clone();
        let selection_event = tree.selection_event.clone();
        let navigation_event = tree.navigation_event.clone();
        if !item.disabled {
            element = element.cursor(gpui::CursorStyle::PointingHand).on_click(
                move |event, window, cx| {
                    if event.modifiers().secondary() {
                        if let Some(open) = open_event.as_deref() {
                            emit_open(&item_runtime, window_id, open, &event_id, event.modifiers());
                            cx.stop_propagation();
                            return;
                        }
                    }
                    item_focus.focus(window, cx);
                    let moved = if let Ok(mut navigation) = item_navigation.lock() {
                        let moved = navigation.focused.as_ref() != Some(&event_id);
                        navigation.focused = Some(event_id.clone());
                        navigation.keyboard = false;
                        moved
                    } else {
                        false
                    };
                    if moved {
                        super::uniform_collection::emit_change(
                            &item_runtime,
                            window_id,
                            navigation_event.as_deref(),
                            &event_id,
                        );
                    }
                    if multiple {
                        let ids = if let Ok(mut navigation) = item_navigation.lock() {
                            let modifiers = event.modifiers();
                            if modifiers.shift {
                                navigation.group.range(
                                    &event_id,
                                    &selection_items,
                                    modifiers.control || modifiers.platform,
                                );
                            } else if modifiers.control || modifiers.platform {
                                navigation.group.toggle(&event_id);
                            } else {
                                navigation.group.single(&event_id);
                            }
                            navigation.group.ids.clone()
                        } else {
                            return;
                        };
                        emit_selection(&item_runtime, window_id, selection_event.as_deref(), &ids);
                        if let Some(last) = ids.last().filter(|last| *last != &event_id) {
                            super::uniform_collection::emit_change(
                                &item_runtime,
                                window_id,
                                Some(&select_event),
                                last,
                            );
                        }
                    }
                    let toggle = branch
                        && if toggle_on_click {
                            !event.modifiers().shift
                                && !event.modifiers().control
                                && !event.modifiers().platform
                        } else {
                            event.click_count() == 2
                        };
                    if !toggle || toggle_on_click {
                        super::uniform_collection::emit_change(
                            &item_runtime,
                            window_id,
                            Some(select_event.as_str()),
                            &event_id,
                        );
                    }
                    if toggle {
                        super::uniform_collection::emit_change(
                            &item_runtime,
                            window_id,
                            Some(toggle_event.as_str()),
                            &event_id,
                        );
                    }
                },
            );
        }
        rendered.push(element.into_any_element());
    }

    component
        .input_entities
        .retain(|input_id, _entity| active_input_ids.contains(input_id));
    component
        .terminal_entities
        .retain(|id, _entity| active_input_ids.contains(id));
    component.components.finish_render(window, cx);

    let input_prefix = format!("gpui-elixir-input-{window_id}-tree-{tree_id}-");
    if let Ok(mut input_values) = runtime.input_values.lock() {
        input_values.retain(|input_id, _value| {
            !input_id.starts_with(&input_prefix) || active_input_ids.contains(input_id)
        });
    }
    rendered
}

#[cfg(feature = "components")]
fn emit_selection(
    runtime: &crate::SharedRuntime,
    window_id: u64,
    event: Option<&str>,
    ids: &[String],
) {
    let Some(event) = event.filter(|value| !value.is_empty()) else {
        return;
    };
    let _ = crate::push_event(
        runtime,
        crate::NativeEvent::Input {
            kind: crate::InputKind::Change,
            window_id,
            event: event.to_string(),
            value: Some(crate::EventValue::Strings(ids.to_vec())),
        },
    );
}

#[cfg(feature = "components")]
fn emit_open(
    runtime: &crate::SharedRuntime,
    window_id: u64,
    event: &str,
    value: &str,
    modifiers: gpui::Modifiers,
) {
    let _ = crate::push_event(
        runtime,
        crate::NativeEvent::ModifiedInput {
            kind: crate::InputKind::Open,
            window_id,
            event: event.to_string(),
            value: Some(crate::EventValue::String(value.to_string())),
            control: modifiers.control,
            alt: modifiers.alt,
            shift: modifiers.shift,
            platform: modifiers.platform,
        },
    );
}

#[cfg(feature = "components")]
#[derive(Clone, Copy)]
enum TreeAction {
    Select(usize),
    Toggle(usize),
}

#[cfg(feature = "components")]
fn key_action(
    key: &str,
    items: &[TreeKey],
    selected: Option<usize>,
    total_count: usize,
) -> Option<TreeAction> {
    match key {
        "left" => left_action(items, selected),
        "right" => right_action(items, selected),
        _other => super::uniform_collection::linear_key_target(
            key,
            items,
            selected,
            total_count,
            |item| item.index,
            |item| item.disabled,
        )
        .map(TreeAction::Select),
    }
}

#[cfg(feature = "components")]
fn left_action(items: &[TreeKey], selected: Option<usize>) -> Option<TreeAction> {
    let selected = selected?;
    let item = &items[selected];
    if item.disabled {
        return None;
    }
    if item.branch && item.expanded {
        return Some(TreeAction::Toggle(selected));
    }
    let parent = item.parent_id.as_ref()?;
    items
        .iter()
        .position(|candidate| candidate.id == *parent && !candidate.disabled)
        .map(TreeAction::Select)
}

#[cfg(feature = "components")]
fn right_action(items: &[TreeKey], selected: Option<usize>) -> Option<TreeAction> {
    let selected = selected?;
    let item = &items[selected];
    if item.disabled || !item.branch {
        return None;
    }
    if !item.expanded {
        return Some(TreeAction::Toggle(selected));
    }
    items
        .iter()
        .enumerate()
        .skip(selected + 1)
        .find(|(_position, candidate)| {
            candidate.level == item.level.saturating_add(1)
                && candidate.parent_id.as_deref() == Some(item.id.as_str())
                && !candidate.disabled
        })
        .map(|(position, _candidate)| TreeAction::Select(position))
}

#[cfg(not(feature = "components"))]
pub(crate) fn render(
    node: TreeComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    render_component_fallback(node.style, node.label, node.children, context)
}

#[cfg(feature = "components")]
pub(crate) fn render_item(
    node: TreeItemComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    use crate::element::apply_generated_render_styles;
    use gpui::{IntoElement, ParentElement};

    let children = node
        .children
        .into_iter()
        .map(|child| child.render(context))
        .collect::<Vec<_>>();
    apply_generated_render_styles(gpui::div(), node.style)
        .children(children)
        .into_any_element()
}

#[cfg(not(feature = "components"))]
pub(crate) fn render_item(
    node: TreeItemComponentNode,
    context: &mut ElementRenderContext<'_, '_>,
) -> gpui::AnyElement {
    render_component_fallback(node.style, None, node.children, context)
}

#[cfg(all(test, feature = "components"))]
mod tests {
    #[cfg(feature = "components")]
    #[test]
    fn open_event_preserves_target_and_pointer_modifiers() {
        let runtime = std::sync::Arc::new(crate::runtime::RuntimeState::new());
        super::emit_open(
            &runtime,
            7,
            "open-file",
            "file-id",
            crate::gpui::Modifiers {
                platform: true,
                shift: true,
                ..Default::default()
            },
        );
        let events = runtime.events.lock().unwrap();
        assert!(matches!(&events[0], crate::NativeEvent::ModifiedInput {
            kind: crate::InputKind::Open, window_id: 7, event,
            value: Some(crate::EventValue::String(value)), platform: true, shift: true,
            control: false, alt: false,
        } if event == "open-file" && value == "file-id"));
    }

    use super::*;

    fn key(
        id: &str,
        parent_id: Option<&str>,
        level: usize,
        branch: bool,
        expanded: bool,
        disabled: bool,
        index: usize,
    ) -> TreeKey {
        TreeKey {
            id: id.to_string(),
            parent_id: parent_id.map(str::to_string),
            level,
            branch,
            expanded,
            disabled,
            index,
        }
    }

    #[test]
    fn tree_keys_expand_and_navigate_hierarchy_while_skipping_disabled_items() {
        let collapsed = vec![
            key("root", None, 1, true, false, false, 0),
            key("disabled", Some("root"), 2, false, false, true, 1),
            key("child", Some("root"), 2, false, false, false, 2),
        ];
        assert!(matches!(
            key_action("right", &collapsed, Some(0), collapsed.len()),
            Some(TreeAction::Toggle(0))
        ));
        assert!(matches!(
            key_action("down", &collapsed, Some(0), collapsed.len()),
            Some(TreeAction::Select(2))
        ));

        let mut expanded = collapsed;
        expanded[0].expanded = true;
        assert!(matches!(
            key_action("right", &expanded, Some(0), expanded.len()),
            Some(TreeAction::Select(2))
        ));
        assert!(matches!(
            key_action("left", &expanded, Some(2), expanded.len()),
            Some(TreeAction::Select(0))
        ));
        assert!(matches!(
            key_action("left", &expanded, Some(0), expanded.len()),
            Some(TreeAction::Toggle(0))
        ));
    }
}
