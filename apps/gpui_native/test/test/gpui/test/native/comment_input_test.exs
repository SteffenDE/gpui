defmodule GPUI.Test.Native.CommentInputTest do
  use GPUI.Test, native: [size: {400, 250}]

  defmodule View do
    use GPUI.View
    alias GPUI.UI
    @impl true
    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-[360px]">
        <UI.input id="comment" label="Comment" multiline={true} submit_on_enter={false} appearance={false}
          value={assigns.text} focus_request={1} capture_keys={["escape"]}
          phx-change="text" phx-submit="submit" phx-keystroke="key" />
        <button id="after" phx-focus="after-focused" phx-click="after-click">After</button>
      </div>
      """
    end
  end

  defmodule InsertView do
    use GPUI.View
    alias GPUI.UI

    def render(assigns) do
      UI.input(%{
        id: "insert-comment",
        label: "Comment",
        multiline: true,
        submit_on_enter: false,
        value: assigns.text,
        focus_request: 1,
        insert_request: Map.get(assigns, :request, 0),
        insert_text: Map.get(assigns, :insert),
        insert_match: Map.get(assigns, :match),
        "phx-inserted": Map.get(assigns, :ack),
        insert_on_new_line: Map.get(assigns, :new_line, false),
        disabled: Map.get(assigns, :disabled, false),
        "phx-change": "insert-text"
      })
    end
  end

  test "requested insertion replaces a Unicode selection once and keeps the insertion cursor", %{
    ui: ui
  } do
    render(ui, InsertView, text: "a😀b")
    settle(ui)
    press(ui, "cmd-right")
    press(ui, "shift-left")
    render(ui, InsertView, text: "a😀b", request: 1, insert: "!")
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "insert-text", value: "a😀!"}}}
    render(ui, InsertView, text: "a😀!", request: 1, insert: "!")
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "insert-text"}}}, 10
    type(ui, "?")
    assert_receive {:gpui, ^ui, {:event, %{event: "insert-text", value: "a😀!?"}}}
  end

  test "a line insertion replaces the selection and typing continues before following text", %{
    ui: ui
  } do
    render(ui, InsertView, text: "left right")
    settle(ui)
    press(ui, "cmd-right")
    for _ <- 1..5, do: press(ui, "left")
    press(ui, "shift-left")
    render(ui, InsertView, text: "left right", request: 1, insert: "asset\n", new_line: true)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "insert-text", value: "left\nasset\nright"}}}

    render(ui, InsertView,
      text: "left\nasset\nright",
      request: 1,
      insert: "asset\n",
      new_line: true
    )

    type(ui, "X")
    assert_receive {:gpui, ^ui, {:event, %{event: "insert-text", value: "left\nasset\nXright"}}}
  end

  test "disabled inputs consume insertion requests without changing the value", %{ui: ui} do
    render(ui, InsertView, text: "kept", request: 1, insert: "ignored", disabled: true)
    settle(ui)
    render(ui, InsertView, text: "kept", request: 1, insert: "ignored", disabled: false)
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "insert-text"}}}, 10
  end

  test "match replacement preserves a cursor before the upload and acknowledges after change", %{
    ui: ui
  } do
    render(ui, InsertView, text: "left [upload] tail")
    settle(ui)
    press(ui, "cmd-left")
    for _ <- 1..3, do: press(ui, "right")

    render(ui, InsertView,
      text: "left [upload] tail",
      request: 1,
      insert: "URL",
      match: "[upload]",
      ack: "inserted"
    )

    settle(ui)
    assert_receive {:gpui, ^ui, {:event, changed}}
    assert changed.event == "insert-text" and changed.value == "left URL tail"
    assert_receive {:gpui, ^ui, {:event, %{event: "inserted", value: ["1", "left URL tail"]}}}

    render(ui, InsertView,
      text: "left URL tail",
      request: 1,
      insert: "URL",
      match: "[upload]",
      ack: "inserted"
    )

    type(ui, "X")
    assert_receive {:gpui, ^ui, {:event, %{event: "insert-text", value: "lefXt URL tail"}}}
  end

  test "a deleted placeholder acknowledges without resurrecting text or moving the cursor", %{
    ui: ui
  } do
    render(ui, InsertView, text: "kept")
    settle(ui)
    press(ui, "cmd-right")

    render(ui, InsertView,
      text: "kept",
      request: 1,
      insert: "URL",
      match: "[upload]",
      ack: "inserted"
    )

    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "inserted", value: ["1", "kept"]}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "insert-text"}}}, 10
    type(ui, "!")
    assert_receive {:gpui, ^ui, {:event, %{event: "insert-text", value: "kept!"}}}
  end

  defmodule RecreatedView do
    use GPUI.View
    alias GPUI.UI
    @impl true
    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-[360px]">
        {if assigns.shown, do: UI.input(%{id: "recreated-comment", label: "Comment", value: "", focus_request: assigns.request, multiline: true, submit_on_enter: false, "phx-change": "recreated-text", "phx-focus": "recreated-focus"}), else: []}
      </div>
      """
    end
  end

  test "a remounted multiline input reports focus for its new native handle", %{ui: ui} do
    render(ui, RecreatedView, shown: true, request: 1)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "recreated-focus"}}}
    render(ui, RecreatedView, shown: false, request: 1)
    settle(ui)
    render(ui, RecreatedView, shown: true, request: 2)
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "recreated-focus"}}}
  end

  test "Enter and Shift-Enter insert newlines, primary Enter submits without editing", %{ui: ui} do
    render(ui, View, text: "")
    settle(ui)
    type(ui, "First")

    for value <- ["F", "Fi", "Fir", "Firs", "First"] do
      assert_receive {:gpui, ^ui, {:event, %{event: "text", value: ^value}}}
    end

    render(ui, View, text: "First")
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "text", value: "First\n"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "submit"}}}, 10
    render(ui, View, text: "First\n")
    press(ui, "shift-enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "text", value: "First\n\n"}}}
    render(ui, View, text: "First\n\n")
    press(ui, if(:os.type() == {:unix, :darwin}, do: "cmd-enter", else: "ctrl-enter"))
    assert_receive {:gpui, ^ui, {:event, %{event: "submit", value: "First\n\n"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "text"}}}, 10
  end

  test "captured Escape is structured and leaves text and focus under caller control", %{ui: ui} do
    render(ui, View, text: "draft")
    settle(ui)
    press(ui, "escape")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{type: :keystroke, event: "key", value: %{key: "escape", platform: false}}}}

    refute_receive {:gpui, ^ui, {:event, %{event: "text"}}}, 10
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "after-focused"}}}
  end
end
