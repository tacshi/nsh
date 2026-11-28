// A case of the API's test suite (docs/dev/compatibility.md): the `table`
// extension's `fullWidth` option in `nsh/base`. Three read-only editors of
// the same width: tables as they are by default (columns fit their
// content), with `fullWidth` (columns without a width share the editor's
// text) and with `fullWidth` and every column sized (the last column takes
// the room left). Its script checks where the cells are laid out.
import { View } from "nsh";
import { RichEditor, StarterKit, TableKit, createRichEditor } from "nsh/base";

const cell = (text: string, width?: number) => ({
  type: "tableCell",
  attrs: width ? { colwidth: [width] } : {},
  content: [{ type: "paragraph", content: [{ type: "text", text }] }],
});

const tableDoc = (cells: ReturnType<typeof cell>[]) => ({
  type: "doc",
  content: [{ type: "table", content: [{ type: "tableRow", content: cells }] }],
});

const full = TableKit.configure({ table: { fullWidth: true } });

function App() {
  const byDefault = createRichEditor({
    extensions: [StarterKit, TableKit],
    content: tableDoc([cell("DefaultA"), cell("DefaultB")]),
  });
  const shared = createRichEditor({
    extensions: [StarterKit, full],
    content: tableDoc([cell("SharedA"), cell("SharedB")]),
  });
  const sized = createRichEditor({
    extensions: [StarterKit, full],
    content: tableDoc([cell("SizedA", 80), cell("SizedB", 80)]),
  });
  const editor = { width: 300, height: 60, flexShrink: 0 };

  return (
    <View style={{ flex: 1, paddingTop: 60, paddingLeft: 20, gap: 4 }}>
      <RichEditor editor={byDefault} editable={false} style={editor} />
      <RichEditor editor={shared} editable={false} style={editor} />
      <RichEditor editor={sized} editable={false} style={editor} />
    </View>
  );
}

export default App;
