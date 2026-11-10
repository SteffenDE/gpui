defmodule GPUI.Test.Native.TreeTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule TreeView do
    use GPUI.View

    alias GPUI.UI

    @impl GPUI.View
    def render(assigns) do
      ~GPUI"""
      <UI.tree
        id="files"
        label="Files"
        selected={assigns.selected}
        multiple={assigns.multiple}
        selected_items={assigns.selected_items}
        selection_items={assigns.selection_items}
        phx-selection-change={assigns.selection_event}
        select_on_navigation={assigns.select_on_navigation}
        toggle_on_click={assigns.toggle_on_click}
        total_count={4}
        offset={0}
        item_height={40}
        phx-change="tree_selected"
        phx-navigation-change="tree_focus"
        phx-toggle="tree_toggled"
        phx-open={assigns.open_event}
        class="h-[200px]"
      >
        <UI.tree_item id="lib" level={1} branch={true} expanded={assigns.expanded}>
          {if assigns.hover_only do
            UI.rich_text(%{id: "folder-label", label: "Folder", text: "lib", selectable: false,
              runs: [GPUI.Text.RichRun.new(GPUI.Text.Range.new(GPUI.Text.Position.new(0, 0), GPUI.Text.Position.new(0, 3)), link: "lib")],
              "phx-link-hover": "span_hover"})
          else
            %GPUI.Element{type: :text, children: ["lib"]}
          end}
        </UI.tree_item>
        <UI.tree_item id="disabled" level={2} parent_id="lib" disabled={true}>
          <text>disabled.ex</text>
        </UI.tree_item>
        <UI.tree_item id="app" level={2} parent_id="lib">
          <text>app.ex</text>
        </UI.tree_item>
        <UI.tree_item id="readme" level={1}>
          <div id="readme-label"><text>README</text></div>
        </UI.tree_item>
      </UI.tree>
      """
    end
  end

  defp render_tree(ui, opts \\ []) do
    render(
      ui,
      TreeView,
      Keyword.merge(
        [
          selected: "lib",
          multiple: false,
          selected_items: [],
          selection_items: [],
          selection_event: nil,
          expanded: false,
          select_on_navigation: true,
          toggle_on_click: false,
          hover_only: false,
          open_event: nil
        ],
        opts
      )
    )
  end

  test "roving focus emits stable IDs without selecting or duplicating activation", %{ui: ui} do
    render_tree(ui, expanded: true, select_on_navigation: false)
    focus(ui, "files")
    press(ui, :arrow_down)
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_focus", value: "app"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "tree_selected"}}}
    press(ui, :enter)
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_selected", value: "app"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "tree_focus"}}}
    click(ui, "readme-label")
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_focus", value: "readme"}}}
  end

  test "controlled multiple selection extends, selects all and toggles without changing focus", %{
    ui: ui
  } do
    opts = [
      selected: "readme",
      expanded: true,
      multiple: true,
      select_on_navigation: false,
      selected_items: ["readme"],
      selection_items: ["lib", "app", "readme"],
      selection_event: "group"
    ]

    render_tree(ui, opts)
    focus(ui, "files")
    press(ui, "home")
    press(ui, "shift-down")
    assert_receive {:gpui, ^ui, {:event, %{event: "group", value: ["readme", "app"]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_selected", value: "app"}}}
    render_tree(ui, Keyword.put(opts, :selected_items, ["readme", "app"]))
    press(ui, "cmd-a")
    assert_receive {:gpui, ^ui, {:event, %{event: "group", value: ["lib", "app", "readme"]}}}
    render_tree(ui, Keyword.put(opts, :selected_items, ["lib", "app", "readme"]))
    press(ui, "ctrl-space")
    assert_receive {:gpui, ^ui, {:event, %{event: "group", value: ["lib", "readme"]}}}
  end

  defmodule VirtualSelectionView do
    use GPUI.View
    alias GPUI.UI

    def render(_assigns) do
      ~GPUI"""
      <UI.tree id="virtual-multiple" label="Sparse files" multiple={true}
        selected="row-100" selected_index={100} selected_items={["row-100"]}
        selection_items={Enum.map(0..299, &"row-#{&1}")}
        total_count={300} offset={100} select_on_navigation={false}
        phx-toggle="sparse-toggle" phx-change="sparse-primary" phx-range="sparse-range" phx-selection-change="sparse-group" class="h-[200px]">
        <UI.tree_item id="row-100" position={101} set_size={300}><text>100</text></UI.tree_item>
        <UI.tree_item id="row-101" position={102} set_size={300}><text>101</text></UI.tree_item>
      </UI.tree>
      """
    end
  end

  test "select all uses the complete selectable order beyond loaded virtual rows", %{ui: ui} do
    render(ui, VirtualSelectionView, %{})
    focus(ui, "virtual-multiple")
    press(ui, "cmd-a")
    assert_receive {:gpui, ^ui, {:event, %{event: "sparse-group", value: values}}}
    assert values == Enum.map(0..299, &"row-#{&1}")
  end

  test "expands branches and skips disabled children", %{ui: ui} do
    render_tree(ui)
    focus(ui, "files")
    press(ui, :arrow_right)

    assert_receive {:gpui, ^ui, {:event, %{type: :change, event: "tree_toggled", value: "lib"}}}

    render_tree(ui, expanded: true)
    focus(ui, "files")
    press(ui, :arrow_right)

    assert_receive {:gpui, ^ui, {:event, %{type: :change, event: "tree_selected", value: "app"}}}
  end

  test "linear navigation skips disabled items", %{ui: ui} do
    render_tree(ui, expanded: true)
    focus(ui, "files")
    press(ui, :arrow_down)

    assert_receive {:gpui, ^ui, {:event, %{type: :change, event: "tree_selected", value: "app"}}}

    render_tree(ui, selected: "app", expanded: true)
    focus(ui, "files")
    press(ui, :arrow_down)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :change, event: "tree_selected", value: "readme"}}}
  end

  test "focus-only arrows retain selection until activation", %{ui: ui} do
    render_tree(ui, expanded: true, select_on_navigation: false)
    focus(ui, "files")
    press(ui, :arrow_down)
    refute_receive {:gpui, ^ui, {:event, %{event: "tree_selected"}}}
    press(ui, :enter)
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_selected", value: "app"}}}
  end

  test "passive spans preserve tree focus and single-click toggling", %{ui: ui} do
    render_tree(ui,
      expanded: true,
      select_on_navigation: false,
      toggle_on_click: true,
      hover_only: true
    )

    click(ui, "folder-label")
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_selected", value: "lib"}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_toggled", value: "lib"}}}
    press(ui, :arrow_down)
    refute_receive {:gpui, ^ui, {:event, %{event: "tree_selected"}}}
    press(ui, :enter)
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_selected", value: "app"}}}
  end

  test "an open callback leaves ordinary Enter selection unchanged", %{ui: ui} do
    render_tree(ui, open_event: "tree_opened", select_on_navigation: false)
    focus(ui, "files")
    press(ui, :enter)
    assert_receive {:gpui, ^ui, {:event, %{event: "tree_selected", value: "lib"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "tree_opened"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "tree_toggled"}}}
  end
end
