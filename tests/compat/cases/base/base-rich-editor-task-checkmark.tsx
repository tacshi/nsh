// A case of the API's test suite (docs/dev/compatibility.md): the
// `RichEditor` view's `taskCheckmark` part in `nsh/base`. A done task
// item's checkbox holds no mark until the app gives the part classes or a
// style; with either, a `✓` in the part's color, gone once the item is open.
import { Text, View, signal } from "nsh";
import { RichEditor, StarterKit, createRichEditor } from "nsh/base";

const MODES = ["none", "classes", "styles"] as const;

function App() {
  const editor = createRichEditor({
    extensions: [StarterKit],
    content: "- [x] TaskDone\n- [ ] TaskOpen\n",
    format: "markdown",
  });
  const mode = signal<(typeof MODES)[number]>("none");
  return (
    <View style={{ flex: 1, paddingTop: 60, paddingLeft: 20, gap: 4 }}>
      <View onPress={() => mode.set(MODES[(MODES.indexOf(mode()) + 1) % MODES.length])}>
        <Text>{`CheckmarkMode ${mode()}`}</Text>
      </View>
      <RichEditor
        editor={editor}
        style={{ height: 120 }}
        classes={{
          taskCheckbox: "size-4 border",
          ...(mode() === "classes" ? { taskCheckmark: "text-[#ff0000]" } : {}),
        }}
        styles={mode() === "styles" ? { taskCheckmark: { color: "#0000ff" } } : {}}
      />
    </View>
  );
}

export default App;
