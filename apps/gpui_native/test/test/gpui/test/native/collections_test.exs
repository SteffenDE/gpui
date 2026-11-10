defmodule GPUI.Test.Native.CollectionsTest do
  use GPUI.Test, native: [size: {360, 240}]

  defmodule ListView do
    use GPUI.View

    alias GPUI.UI

    @impl GPUI.View
    def render(assigns) do
      ~GPUI"""
      <UI.virtual_list
        id="records"
        label="Records"
        selected={assigns.selected}
        item_height={40}
        phx-change="record_selected"
        class="h-[200px]"
      >
        {Enum.map(assigns.items, &item/1)}
      </UI.virtual_list>
      """
    end

    defp item(id) do
      ~GPUI"""
      <UI.virtual_list_item id={id} disabled={id == "item-2"}>
        <div class="h-[40px]"><text>{id}</text></div>
      </UI.virtual_list_item>
      """
    end
  end

  defmodule SourceListView do
    use GPUI.View

    alias GPUI.UI

    @impl GPUI.View
    def render(assigns) do
      first = assigns.range.first
      last = min(assigns.range.last, assigns.total_count)
      items = if first < last, do: Enum.map(first..(last - 1), &item/1), else: []

      ~GPUI"""
      <UI.virtual_list
        id="source-records"
        label="Source records"
        selected={assigns.selected}
        selected_index={assigns.selected_index}
        total_count={assigns.total_count}
        offset={first}
        overscan={2}
        item_height={40}
        phx-change="source_selected"
        phx-range="source_range"
        class="h-[200px]"
      >
        {items}
      </UI.virtual_list>
      """
    end

    defp item(index) do
      id = "row-#{index}"

      ~GPUI"""
      <UI.virtual_list_item id={id}>
        <div class="h-[40px]"><text>{id}</text></div>
      </UI.virtual_list_item>
      """
    end
  end

  defmodule VariableView do
    use GPUI.View

    alias GPUI.UI

    @impl GPUI.View
    def render(assigns) do
      ~GPUI"""
      <UI.virtual_collection
        id="variable-records"
        label="Variable records"
        measure_all={true}
        scrollbar={Map.get(assigns, :scrollbar, false)}
        reveal={Map.get(assigns, :reveal)}
        reveal_request={Map.get(assigns, :reveal_request, 0)}
        reveal_strategy="top"
        reveal_offset={Map.get(assigns, :reveal_offset, 0.0)}
        follow={assigns.follow}
        follow_request={assigns.follow_request}
        phx-range="variable_range"
        phx-scroll="variable_scroll"
        phx-measure={Map.get(assigns, :measure_event)}
        class="h-[200px]"
      >
        {Enum.map(assigns.items, &item/1)}
      </UI.virtual_collection>
      """
    end

    defp item({id, revision, height}) do
      ~GPUI"""
      <UI.virtual_item id={id} revision={revision} style={[height: {:px, height}]}>
        <div id={id <> "-label"} class="w-full h-full"><text>{id}</text></div>
      </UI.virtual_item>
      """
    end
  end

  test "variable collections report native item sizes and suppress unchanged measurements", %{
    ui: ui
  } do
    attrs = [
      items: [{"one", 0, 40}, {"two", 0, 80}],
      follow: "none",
      follow_request: 0,
      measure_event: "item_size"
    ]

    render(ui, VariableView, attrs)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{
                       type: :bounds,
                       event: "item_size",
                       value: %{id: "one", width: width, height: 40.0}
                     }}}

    assert width > 0

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{type: :bounds, event: "item_size", value: %{id: "two", height: 80.0}}}}

    render(ui, VariableView, attrs)
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "item_size"}}}, 20
    render(ui, VariableView, Keyword.put(attrs, :items, [{"one", 1, 100}, {"two", 0, 80}]))
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{type: :bounds, event: "item_size", value: %{id: "one", height: 100.0}}}}
  end

  test "vertical scrollbar reserves its lane, pages, and drags variable rows", %{ui: ui} do
    render(ui, VariableView,
      items: Enum.map(1..20, &{"item-#{&1}", 0, 40}),
      follow: "none",
      follow_request: 0,
      scrollbar: true
    )

    settle(ui)
    collection = bounds(ui, "variable-records")
    row = bounds(ui, "item-1-label")
    assert_in_delta row.width, collection.width - 10, 0.1
    x = collection.x + collection.width - 5
    drag(ui, {x - 20, collection.y + 180}, {x, collection.y + 180})
    click(ui, {x, collection.y + 180})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [index, offset]}}}
                   when index == 5 and offset == 0

    # 800px of content in a 200px viewport: the thumb is 50px high,
    # and a 50px thumb drag moves the content by 200px.
    drag(ui, {x, collection.y + 75}, {x, collection.y + 125})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [index, _]}}}
                   when index >= 9

    scroll(ui, "variable-records", delta: {0, 40})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [index, _]}}}
                   when index < 10
  end

  test "uniform list keyboard navigation skips disabled items", %{ui: ui} do
    render(ui, ListView, selected: "item-1", items: Enum.map(1..20, &"item-#{&1}"))
    focus(ui, "records")
    press(ui, :arrow_down)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :change, event: "record_selected", value: "item-3"}}}

    render(ui, ListView, selected: "item-3", items: Enum.map(1..20, &"item-#{&1}"))
    focus(ui, "records")
    press(ui, :arrow_up)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :change, event: "record_selected", value: "item-1"}}}
  end

  test "source-backed list emits bounded ranges after scrolling", %{ui: ui} do
    render(ui, SourceListView,
      selected: "row-0",
      selected_index: 0,
      total_count: 1_000,
      range: %{first: 0, last: 20}
    )

    assert_receive {:gpui, ^ui, {:event, %{type: :range, event: "source_range", value: initial}}}
    assert initial.first == 0
    assert initial.last <= 20

    scroll(ui, "source-records", delta: {0, -240})

    assert_receive {:gpui, ^ui, {:event, %{type: :range, event: "source_range", value: range}}}
    assert range.first > initial.first
    assert range.first < range.last
    assert range.last <= 1_000
  end

  test "variable collections report the top item and its pixel offset", %{ui: ui} do
    render(ui, VariableView,
      items: Enum.map(1..8, &{"item-#{&1}", 0, 40}),
      follow: "none",
      follow_request: 0
    )

    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :change, event: "variable_scroll", value: [index, offset]}}}

    assert index == 0 and offset == 0
    scroll(ui, "variable-records", delta: {0, -55})
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :change, event: "variable_scroll", value: [index, offset]}}}

    assert index == 1 and offset == 15
  end

  test "reveals retain a requested pixel offset after preceding insertion", %{ui: ui} do
    items = Enum.map(1..8, &{"item-#{&1}", 0, 40})

    render(ui, VariableView,
      items: items,
      follow: "none",
      follow_request: 0,
      reveal: "item-3",
      reveal_request: 1,
      reveal_offset: 7.5
    )

    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [index, offset]}}}
                   when index == 2 and offset == 7.5

    render(ui, VariableView,
      items: [{"inserted", 0, 40} | items],
      follow: "none",
      follow_request: 0,
      reveal: "item-3",
      reveal_request: 2,
      reveal_offset: 7.5
    )

    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [index, offset]}}}
                   when index == 3 and offset == 7.5
  end

  test "negative top offsets place a revealed row below the viewport top using measured preceding heights",
       %{ui: ui} do
    items = Enum.map(1..20, &{"item-#{&1}", 0, if(rem(&1, 2) == 0, do: 60, else: 30)})

    attrs = [
      items: items,
      follow: "none",
      follow_request: 0,
      reveal: "item-8",
      reveal_request: 1,
      reveal_offset: -85.0
    ]

    render(ui, VariableView, attrs)
    settle(ui)
    assert_in_delta bounds(ui, "item-8-label").y, 85.0, 0.1
    refute_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [7, +0.0]}}}, 20

    items = List.replace_at(items, 6, {"item-7", 1, 100})
    render(ui, VariableView, Keyword.merge(attrs, items: items, reveal_request: 2))
    settle(ui)
    assert_in_delta bounds(ui, "item-8-label").y, 85.0, 0.1
    refute_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [7, +0.0]}}}, 20

    render(ui, VariableView, Keyword.merge(attrs, reveal: "item-1", reveal_request: 3))
    settle(ui)
    assert_in_delta bounds(ui, "item-1-label").y, 0.0, 0.1
  end

  test "exact measurement survives inserting items after an empty frame", %{ui: ui} do
    render(ui, VariableView, items: [], follow: "none", follow_request: 0)
    settle(ui)

    render(ui, VariableView,
      items: Enum.map(1..8, &{"inserted-#{&1}", 1, 60}),
      follow: "none",
      follow_request: 0
    )

    settle(ui)
    scroll(ui, "variable-records", delta: {0, -255})
    settle(ui)

    assert_receive {:gpui, ^ui, {:event, %{event: "variable_scroll", value: [index, offset]}}}
                   when index == 4 and offset == 15
  end

  test "variable collections survive empty, populated, resized, and empty transitions", %{ui: ui} do
    render(ui, VariableView, items: [], follow: "none", follow_request: 0)
    assert %{width: width, height: height} = bounds(ui, "variable-records")
    assert width > 0
    assert height > 0

    render(ui, VariableView,
      items: [{"one", 0, 32}, {"two", 0, 64}, {"three", 0, 48}],
      follow: "none",
      follow_request: 0
    )

    settle(ui)
    scroll(ui, "variable-records", delta: {0, -80})

    render(ui, VariableView,
      items: [{"one", 1, 80}, {"two", 0, 64}, {"three", 0, 48}, {"four", 0, 40}],
      follow: "tail",
      follow_request: 1
    )

    settle(ui)
    assert %{width: resized_width, height: resized_height} = bounds(ui, "variable-records")
    assert resized_width > 0
    assert resized_height > 0

    render(ui, VariableView, items: [], follow: "none", follow_request: 1)
    settle(ui)
    assert %{width: final_width, height: final_height} = bounds(ui, "variable-records")
    assert final_width > 0
    assert final_height > 0
  end
end
