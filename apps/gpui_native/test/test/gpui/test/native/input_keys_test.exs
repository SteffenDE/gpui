defmodule GPUI.Test.Native.InputKeysTest do
  use GPUI.Test, native: [size: {320, 80}]

  defmodule View do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <text_input id="keys" value="" focus_request={1} phx-keydown="legacy" phx-keystroke="structured"
        phx-keyrelease="released" class="w-[300px] h-8" />
      """
    end
  end

  test "structured modifiers coexist with legacy string callbacks", %{ui: ui} do
    render(ui, View, %{})
    settle(ui)
    press(ui, "shift-enter")
    assert_receive {:gpui, ^ui, {:event, %{type: :keydown, event: "legacy", value: legacy}}}
    assert is_binary(legacy)

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{
                       type: :keystroke,
                       event: "structured",
                       value: %{
                         key: "enter",
                         shift: true,
                         control: false,
                         platform: false,
                         repeat: false
                       }
                     }}}
  end

  defmodule CursorView do
    use GPUI.View

    def render(_) do
      ~GPUI"""
      <text_input id="cursor" value="garden" focus_request={1} select_all_request={1}
        phx-change="cursor_changed" class="w-[300px] h-8" />
      """
    end
  end

  test "Command arrows move and extend the native input selection", %{ui: ui} do
    render(ui, CursorView, %{})
    settle(ui)
    press(ui, "cmd-right")
    type(ui, "!")
    assert_receive {:gpui, ^ui, {:event, %{event: "cursor_changed", value: "garden!"}}}
    press(ui, "cmd-shift-left")
    type(ui, "fresh")
    assert_receive {:gpui, ^ui, {:event, %{event: "cursor_changed", value: "fresh"}}}
    press(ui, "cmd-left")
    type(ui, "+")
    assert_receive {:gpui, ^ui, {:event, %{event: "cursor_changed", value: "+fresh"}}}
    press(ui, "cmd-shift-right")
    type(ui, "done")
    assert_receive {:gpui, ^ui, {:event, %{event: "cursor_changed", value: "+done"}}}
  end

  defmodule SurfaceView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <text_surface id="surface" buffer={assigns.buffer} focus_request={1} line_numbers={true}
        phx-keystroke="surface-key" phx-keyrelease="surface-release"
        phx-viewport-change="surface-viewport"
        class="w-[300px] h-16" />
      """
    end
  end

  test "text surfaces report modifier-bearing keystrokes while retaining editing", %{ui: ui} do
    {:ok, buffer} = GPUI.Text.Buffer.new("")
    render(ui, SurfaceView, %{buffer: buffer})
    settle(ui)
    type(ui, "draft")
    assert {:ok, %{text: "draft"}} = GPUI.Text.Buffer.snapshot(buffer)
    press(ui, "shift-e")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{
                       type: :keystroke,
                       event: "surface-key",
                       value: %{key: "e", shift: true, platform: false}
                     }}}

    assert {:ok, %{text: text}} = GPUI.Text.Buffer.snapshot(buffer)
    assert text == "draftE"
  end

  test "initial selections beyond the viewport are revealed after layout", %{ui: ui} do
    alias GPUI.Text.{Buffer, Position, Selection}
    text = Enum.map_join(1..100, "\n", &"line #{&1}")

    {:ok, buffer} =
      Buffer.new(text,
        selections: [Selection.caret("primary", Position.new(75, 0), primary: true)]
      )

    render(ui, SurfaceView, %{buffer: buffer})
    settle(ui)
    advance(ui, 32)
    settle(ui)

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{
                       event: "surface-viewport",
                       value: %{first_visible_row: first, last_visible_row: last}
                     }}}
                   when first > 0 and first <= 75 and last >= 75
  end

  defmodule SelectView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <GPUI.UI.select id="revision" label="Revision" options={assigns.options} value={assigns.value}
        phx-change="revision_changed" class="w-[300px]" />
      """
    end
  end

  test "controlled selects follow external values as options change", %{ui: ui} do
    options = [{"Main", "main"}, {"Maintenance", "maintenance"}, {"Tag", "tag"}]
    render(ui, SelectView, %{options: options, value: "main"})
    settle(ui)
    render(ui, SelectView, %{options: [{"Commit", "commit"} | options], value: "commit"})
    settle(ui)
    render(ui, SelectView, %{options: options, value: "tag"})
    settle(ui)
    click(ui, {100, 16})
    press(ui, :arrow_up)
    press(ui, :arrow_up)
    press(ui, :enter)
    assert_receive {:gpui, ^ui, {:event, %{event: "revision_changed", value: "main"}}}
  end

  defmodule RootCaptureView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div id="capture-root" phx-keycapture={if assigns.enabled, do: "capture"} phx-window-keyrelease={if assigns.enabled, do: "window-release"} capture_keys={["r", "enter", "shift-c", "shift-enter"]} class="flex w-full h-full">
        <text_input id="capture-input" value="" phx-change="typed" class="w-[150px] h-8" />
        <button id="capture-button" phx-focus="button-focused" phx-click="clicked" class="w-[100px] h-8">Button</button>
      </div>
      """
    end
  end

  test "root key capture intercepts listed keys and preserves unmatched keys and input editing",
       %{ui: ui} do
    render(ui, RootCaptureView, %{enabled: true})
    settle(ui)
    focus(ui, "capture-button")
    press(ui, "enter")

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "capture", value: %{key: "enter", captured: true}}}}

    refute_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}
    press(ui, "w")

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "capture", value: %{key: "w", captured: false}}}}

    press(ui, "secondary-r")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{event: "capture", value: %{key: "r", platform: true, captured: false}}}}

    click(ui, {60, 16})
    press(ui, "r")

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "capture", value: %{key: "r", captured: false}}}}

    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "r"}}}
    render(ui, RootCaptureView, %{enabled: false})
    settle(ui)
    focus(ui, "capture-button")
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "capture"}}}
  end

  test "explicit Shift capture suppresses activation and leaves editing and unmatched Shift keys intact",
       %{ui: ui} do
    render(ui, RootCaptureView, %{enabled: true})
    settle(ui)
    focus(ui, "capture-button")
    press(ui, "shift-enter")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{event: "capture", value: %{key: "enter", shift: true, captured: true}}}}

    refute_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}
    press(ui, "shift-c")

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "capture", value: %{key: "c", shift: true, captured: true}}}}

    press(ui, "shift-r")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{event: "capture", value: %{key: "r", shift: true, captured: false}}}}

    click(ui, {60, 16})
    press(ui, "shift-c")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{event: "capture", value: %{key: "c", shift: true, captured: false}}}}

    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "C"}}}
  end

  test "window release observes focused descendants without changing their native actions", %{
    ui: ui
  } do
    render(ui, RootCaptureView, %{enabled: true})
    settle(ui)
    focus(ui, "capture-button")
    press(ui, "shift-c")

    assert_receive {:gpui, ^ui,
                    {:event,
                     %{event: "window-release", value: %{key: "c", shift: true, repeat: false}}}}

    refute_receive {:gpui, ^ui, {:event, %{event: "window-release"}}}
    click(ui, {60, 16})
    press(ui, "c")
    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "c"}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "window-release", value: %{key: "c"}}}}
    render(ui, RootCaptureView, %{enabled: false})
    settle(ui)
    focus(ui, "capture-button")
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{event: "clicked"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "window-release"}}}
  end

  defmodule RegionView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div id="region" focus_request={1} phx-keyrelease="region-key" class="flex w-full h-full gap-2">
        <text_input id="inside-input" value="" phx-focus="input-focused" phx-change="typed" class="w-[160px] h-8" />
        <button id="inside-button" phx-focus="button-focused" phx-click="activated" class="w-[100px] h-8">Activate</button>
      </div>
      """
    end
  end

  test "region releases run only while the region itself owns focus", %{ui: ui} do
    render(ui, RegionView, %{})
    settle(ui)
    focus(ui, "region")
    press(ui, "enter")

    assert_receive {:gpui, ^ui,
                    {:event, %{type: :keystroke, event: "region-key", value: %{key: "enter"}}}}

    focus(ui, "inside-button")
    press(ui, "enter")
    assert_receive {:gpui, ^ui, {:event, %{type: :click, event: "activated"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "region-key"}}}
    click(ui, {60, 16})
    type(ui, "q")
    assert_receive {:gpui, ^ui, {:event, %{event: "typed", value: "q"}}}
    press(ui, "enter")
    refute_receive {:gpui, ^ui, {:event, %{event: "region-key"}}}
  end

  defmodule CaptureView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <text_input id="capture-input" value="x" focus_request={1} capture_keys={["tab", "backspace"]}
        phx-keystroke="captured" phx-keydown="legacy-captured" phx-change="capture-typed" class="w-[300px] h-8" />
      """
    end
  end

  test "captured input keys emit once before native deletion and focus traversal", %{ui: ui} do
    render(ui, CaptureView, %{})
    settle(ui)
    press(ui, "backspace")
    assert_receive {:gpui, ^ui, {:event, %{event: "captured", value: %{key: "backspace"}}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "legacy-captured", value: "backspace"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "captured"}}}
    type(ui, "y")
    assert_receive {:gpui, ^ui, {:event, %{event: "capture-typed", value: "xy"}}}
    press(ui, "tab")
    assert_receive {:gpui, ^ui, {:event, %{event: "captured", value: %{key: "tab"}}}}
    type(ui, "z")
    assert_receive {:gpui, ^ui, {:event, %{event: "capture-typed", value: "xyz"}}}
  end

  defmodule ReverseCaptureView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div class="flex w-full h-full">
        <text_input id="reverse-capture" value="x" focus_request={1} capture_keys={["shift-tab"]}
          phx-keystroke="reverse-key" phx-change="reverse-typed" class="w-[180px] h-8" />
        <button id="next-focus" phx-focus="next-focused" class="w-[100px] h-8">Next</button>
      </div>
      """
    end
  end

  test "explicit Shift-Tab capture keeps input focus without capturing plain Tab", %{ui: ui} do
    render(ui, ReverseCaptureView, %{})
    settle(ui)
    press(ui, "shift-tab")

    assert_receive {:gpui, ^ui,
                    {:event, %{event: "reverse-key", value: %{key: "tab", shift: true}}}}

    type(ui, "y")
    assert_receive {:gpui, ^ui, {:event, %{event: "reverse-typed", value: "xy"}}}
    press(ui, :tab)
    assert_receive {:gpui, ^ui, {:event, %{event: "next-focused"}}}
  end

  defmodule CompletionView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <text_input id="completion" value={assigns.value} focus_request={1} phx-change="completed" class="w-[300px] h-8" />
      """
    end
  end

  test "a caret at the end follows programmatic completion", %{ui: ui} do
    render(ui, CompletionView, %{value: "com"})
    settle(ui)
    render(ui, CompletionView, %{value: "common-"})
    settle(ui)
    type(ui, "a/")
    assert_receive {:gpui, ^ui, {:event, %{event: "completed", value: "common-a/"}}}
  end

  defmodule MovingInputView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full h-full">
        {if assigns.header, do: ~GPUI"<text>Inserted header</text>", else: []}
        <text_input id="moving-input" value={assigns.value} focus_request={1} select_all_request={1}
          phx-change="moving-typed" class="w-[300px] h-8" />
      </div>
      """
    end
  end

  test "explicit input IDs preserve pending edits, caret and focus across sibling insertion", %{
    ui: ui
  } do
    render(ui, MovingInputView, %{header: false, value: "garden"})
    settle(ui)
    press(ui, "cmd-right")
    press(ui, "left")
    press(ui, "left")
    type(ui, "X")
    assert_receive {:gpui, ^ui, {:event, %{event: "moving-typed", value: "gardXen"}}}

    render(ui, MovingInputView, %{header: true, value: "garden"})
    settle(ui)
    type(ui, "Y")
    assert_receive {:gpui, ^ui, {:event, %{event: "moving-typed", value: "gardXYen"}}}

    render(ui, MovingInputView, %{header: false, value: "gardXYen"})
    settle(ui)
    type(ui, "Z")
    assert_receive {:gpui, ^ui, {:event, %{event: "moving-typed", value: "gardXYZen"}}}
  end

  defmodule PointerPositionView do
    use GPUI.View

    def render(_) do
      ~GPUI"""
      <div class="w-full h-full">
        <div id="position-box" phx-pointer="pointer-hover" phx-pointer-position="pointer-position" class="w-[100px] h-8 bg-[#202327]"><text>Pointer</text></div>
      </div>
      """
    end
  end

  test "pointer positions report window coordinates only inside the observed container", %{ui: ui} do
    render(ui, PointerPositionView, %{})
    settle(ui)
    drag(ui, {20, 16}, {25, 18})
    assert_receive {:gpui, ^ui, {:event, %{event: "pointer-position", value: [25.0, 18.0]}}}
    assert_receive {:gpui, ^ui, {:event, %{event: "pointer-hover", value: %{hovered: true}}}}
    press(ui, "e")
    settle(ui)
    refute_receive {:gpui, ^ui, {:event, %{event: "pointer-hover", value: %{hovered: false}}}}, 20
    drag(ui, {200, 50}, {210, 50})
    refute_receive {:gpui, ^ui, {:event, %{event: "pointer-position", value: [210.0, 50.0]}}}, 20
  end

  defmodule ToggleView do
    use GPUI.View

    def render(_assigns) do
      ~GPUI"""
      <div id="settings-switch" accessibility_role="switch" accessibility_checked={true}
        phx-click="toggled" phx-focus="switch-focused" class="w-7 h-4" />
      """
    end
  end

  test "switches activate exactly once per pointer or unmodified keyboard gesture", %{ui: ui} do
    render(ui, ToggleView, %{})
    settle(ui)
    click(ui, {10, 8})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "toggled"}}}
    refute_receive {:gpui, ^ui, {:event, %{event: "toggled"}}}

    for key <- ["space", "enter"] do
      press(ui, key)
      assert_receive {:gpui, ^ui, {:event, %{event: "toggled"}}}
      refute_receive {:gpui, ^ui, {:event, %{event: "toggled"}}}
    end

    press(ui, "ctrl-enter")
    refute_receive {:gpui, ^ui, {:event, %{event: "toggled"}}}
  end

  defmodule NameEditorView do
    use GPUI.View

    def render(assigns) do
      ~GPUI"""
      <div class="flex flex-col w-full h-full">
        {if assigns.open, do: input(assigns), else: []}
        <div id="outside-name-editor" phx-keyrelease="outside-key" class="h-8 w-full"><text>Outside</text></div>
      </div>
      """
    end

    defp input(assigns) do
      ~GPUI"""
      <text_input id="name-editor" value={assigns.value} focus_request={assigns.request} select_all_request={assigns.request}
        phx-change="name-typed" phx-focus="name-focused" phx-blur="name-blurred" class="w-[300px] h-8" />
      """
    end
  end

  test "name editors select once and emit blur again after remounting", %{ui: ui} do
    render(ui, NameEditorView, %{open: true, value: "Inbox", request: 1})
    settle(ui)
    type(ui, "Reviews")
    assert_receive {:gpui, ^ui, {:event, %{event: "name-typed", value: "Reviews"}}}
    click(ui, {20, 50})
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "name-blurred"}}}
    render(ui, NameEditorView, %{open: false, value: "Reviews", request: 1})
    settle(ui)
    render(ui, NameEditorView, %{open: true, value: "Reviews", request: 2})
    settle(ui)
    type(ui, "Issues")
    assert_receive {:gpui, ^ui, {:event, %{event: "name-typed", value: "Issues"}}}
    focus(ui, "outside-name-editor")
    settle(ui)
    assert_receive {:gpui, ^ui, {:event, %{event: "name-blurred"}}}
    focus(ui, "name-editor")
    settle(ui)
    type(ui, "!")
    assert_receive {:gpui, ^ui, {:event, %{event: "name-typed", value: "Issues!"}}}
  end
end
