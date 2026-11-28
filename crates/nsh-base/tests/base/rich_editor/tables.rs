//! Tables in the view: each table's resize handles follow the table, when
//! a block before it grows and when the box it is in (a quote) moves with
//! no change of the table's own frame in it. A grip pressed and let go
//! reports its row or column; dragged, it moves them and reports nothing.

use nsh_base::rich_editor::{
    extensions::{Extension, ExtensionManager, kit},
    tables::commands::set_column_width,
    view::RichEditorTableMenu,
};
use nsh_testing::{DragOptions, Screen};

use super::*;

fn cell(text: &str) -> Value {
    json!({"type": "tableCell", "content": [
        {"type": "paragraph", "content": [{"type": "text", "text": text}]}
    ]})
}

fn table(rows: &[&[&str]]) -> Value {
    let rows: Vec<Value> = rows
        .iter()
        .map(|row| json!({"type": "tableRow", "content": row.iter().map(|t| cell(t)).collect::<Vec<_>>()}))
        .collect();
    json!({"type": "table", "content": rows})
}

/// A paragraph, a table, and a quote holding another table.
fn table_session() -> EditorSession {
    session_with_content(json!([
        {"type": "paragraph", "content": [{"type": "text", "text": "above"}]},
        table(&[&["a1", "a2"], &["b1", "b2"]]),
        {"type": "blockquote", "content": [table(&[&["q1", "q2"]])]},
    ]))
}

fn session_with_content(content: Value) -> EditorSession {
    session_with_kit(content, kit::table_kit())
}

fn session_with_kit(content: Value, tables: Extension) -> EditorSession {
    let manager = ExtensionManager::with_core(vec![kit::starter_kit(), tables]).unwrap();
    let schema = manager.schema().clone();
    let doc = schema
        .node_from_json(&json!({"type": "doc", "content": content}))
        .unwrap();
    let state = EditorState::create(
        EditorStateConfig::new(schema)
            .doc(doc)
            .plugins(manager.plugins()),
    )
    .unwrap();
    EditorSession::new(state, NodeViews::new())
}

/// The editor, and `grow`, which wraps the first paragraph over more lines.
#[component]
fn Tables(props: HarnessProps) -> impl IntoElement {
    let session = props.session.clone().expect("a session");
    let grow = session.clone();
    let grown = signal(false);
    view()
        .child(
            view()
                .test_id("grow")
                .on_press(move || {
                    let state = grow.state();
                    let mut tr = state.tr();
                    tr.insert_text(&"wrapping words ".repeat(40), Some(1), Some(1));
                    grow.dispatch(tr).unwrap();
                    grown.set(true);
                })
                .child(text("Grow")),
        )
        .child(
            view()
                .test_id("editor")
                .w(px(600.0))
                .h(px(600.0))
                .child(rich_editor().editor(session).size_full()),
        )
        .child(text(move || if grown.get() { "Grown" } else { "Ready" }))
}

/// The resize handles' bands, as laid out, top to bottom: a column's is 6
/// points wide down its table, a row's 6 high across it.
fn bands(screen: &Screen) -> Vec<Bounds<Points>> {
    let mut out: Vec<Bounds<Points>> = screen
        .layout_nodes()
        .iter()
        .map(|n| n.bounds)
        .filter(|b| (b.size.width.0 == 6.0) != (b.size.height.0 == 6.0))
        .collect();
    out.sort_by(|a, b| {
        (a.origin.y.0, a.origin.x.0)
            .partial_cmp(&(b.origin.y.0, b.origin.x.0))
            .unwrap()
    });
    out
}

/// The top of the text reading `text`, as laid out.
fn top(screen: &Screen, text: &str) -> f32 {
    screen
        .layout_nodes()
        .iter()
        .find(|n| n.text.as_ref().is_some_and(|t| t.text.text == text))
        .map(|n| n.bounds.origin.y.0)
        .unwrap_or_else(|| panic!("no text reads {text:?}"))
}

/// A one-row table of `s1`, `s2`, ... whose columns have the given widths.
fn sized_row(widths: &[Option<u32>]) -> Value {
    let names: Vec<String> = (1..=widths.len()).map(|i| format!("s{i}")).collect();
    let row: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut t = table(&[&row]);
    for (c, width) in t["content"][0]["content"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(widths)
    {
        if let Some(w) = width {
            c["attrs"] = json!({"colwidth": [w]});
        }
    }
    t
}

/// The harness showing `table` in an editor whose tables are `fullWidth`,
/// once its last cell is laid out, and its session.
fn full_width_app(table: Value) -> (Headless, EditorSession) {
    let last = table["content"][0]["content"].as_array().unwrap().len();
    let session = session_with_kit(
        json!([table]),
        kit::table_kit().configure(json!({"table": {"fullWidth": true}})),
    );
    let held = session.clone();
    let mut config = EngineConfig {
        viewport: Size::new(px(800.0), px(900.0)),
        ..EngineConfig::new(nsh_testing::windowed_root_with::<Tables>(move || {
            HarnessProps {
                session: Some(session.clone()),
            }
        }))
    };
    // A desktop, where columns have resize handles.
    config.environment.os = Os::Linux;
    config.environment.idiom = Idiom::Desktop;
    let mut app = Headless::start(config).unwrap();
    app.wait_for(&format!("text:s{last}")).unwrap();
    app.wait_until("the handles", Duration::from_secs(5), |s| {
        column_bands(s).len() == last
    })
    .unwrap();
    (app, held)
}

fn x_of(screen: &Screen, text: &str) -> f32 {
    screen
        .layout(&format!("text:{text}"))
        .unwrap()
        .unwrap()
        .bounds
        .left()
        .0
}

fn table_frame(screen: &Screen) -> Bounds<Points> {
    screen
        .layout("id:rich-editor-table-scroll")
        .unwrap()
        .unwrap()
        .bounds
}

/// The columns' resize bands, left to right: their views and bounds.
fn column_bands(screen: &Screen) -> Vec<(String, Bounds<Points>)> {
    let mut out: Vec<(String, Bounds<Points>)> = screen
        .layout_nodes()
        .iter()
        .filter(|n| n.bounds.size.width.0 == 6.0 && n.bounds.size.height.0 != 6.0)
        .map(|n| (format!("view:{}", n.id), n.bounds))
        .collect();
    out.sort_by(|a, b| a.1.origin.x.0.partial_cmp(&b.1.origin.x.0).unwrap());
    out
}

/// The table's right edge: the middle of its last column's band.
fn table_edge(screen: &Screen) -> f32 {
    let (_, b) = column_bands(screen).pop().expect("no column band");
    b.origin.x.0 + b.size.width.0 * 0.5
}

/// Drags column `col`'s resize band by `dx`.
fn drag_band(app: &mut Headless, col: usize, dx: f32) {
    let (band, _) = column_bands(app.screen()).swap_remove(col);
    app.drag(&band, Point::new(px(dx), px(0.0)), DragOptions::default())
        .unwrap();
}

#[test]
fn table_columns_fit_their_content_and_scroll_without_moving_the_document() {
    let mut schedule = table(&[
        &["ID", "Task", "Time", "Room", "Host", "Access", "End"],
        &[
            "7",
            "ConferenceRegistrationConfirmation",
            "ScheduledArrivalTime",
            "MainConferenceAuditorium",
            "InternationalRelationsCoordinator",
            "AccessibleTransportationReservation",
            "A",
        ],
    ]);
    for row in schedule["content"].as_array_mut().unwrap() {
        row["content"][2]["attrs"] = json!({"colwidth": [118]});
    }
    let session = session_with_content(json!([
        {"type": "paragraph", "content": [{"type": "text", "text": "Event schedule"}]},
        schedule,
    ]));
    let original = session.state().doc().to_json();
    let held = session.clone();
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(px(800.0), px(900.0)),
        ..EngineConfig::new(nsh_testing::windowed_root_with::<Tables>(move || {
            HarnessProps {
                session: Some(session.clone()),
            }
        }))
    })
    .unwrap();
    let frame = "id:rich-editor-table-scroll";
    app.wait_for(frame).unwrap();
    let bounds = |s: &Screen, selector: &str| s.layout(selector).unwrap().unwrap().bounds;
    app.wait_for("text:End").unwrap();
    let s = app.screen();
    let short = bounds(s, "text:Task").left() - bounds(s, "text:ID").left();
    let long = bounds(s, "text:Time").left() - bounds(s, "text:Task").left();
    assert!(
        long > short,
        "long entries should get more space: {long:?} vs {short:?}"
    );
    let manual = bounds(s, "text:Room").left() - bounds(s, "text:Time").left();
    assert!(
        (manual.0 - 118.0).abs() < 1.,
        "saved column width: {manual:?}"
    );
    let viewport = bounds(s, frame);
    let title = bounds(s, "text:Event schedule");
    assert!(
        bounds(s, "text:End").left() >= viewport.right(),
        "the last column starts offscreen"
    );

    app.scroll(frame, px(10_000.)).unwrap();
    app.wait_until(
        "the last column is visible after a horizontal scroll",
        Duration::from_secs(5),
        |s| {
            let last = bounds(s, "text:End");
            last.left() >= viewport.left() && last.right() <= viewport.right()
        },
    )
    .unwrap();
    assert_eq!(bounds(app.screen(), "text:Event schedule"), title);
    assert_eq!(held.state().doc().to_json(), original);
}

#[test]
fn a_full_width_tables_columns_share_the_editors_width_and_wide_ones_still_scroll() {
    let column_starts = |full: bool, rows: &[&[&str]]| {
        let tables = if full {
            kit::table_kit().configure(json!({"table": {"fullWidth": true}}))
        } else {
            kit::table_kit()
        };
        let session = session_with_kit(json!([table(rows)]), tables);
        let mut app = Headless::start(EngineConfig {
            viewport: Size::new(px(800.0), px(900.0)),
            ..EngineConfig::new(nsh_testing::windowed_root_with::<Tables>(move || {
                HarnessProps {
                    session: Some(session.clone()),
                }
            }))
        })
        .unwrap();
        let first = rows[0][0];
        let last = rows[0][rows[0].len() - 1];
        app.wait_for(&format!("text:{last}")).unwrap();
        let s = app.screen();
        let left = |text: &str| {
            s.layout(&format!("text:{text}"))
                .unwrap()
                .unwrap()
                .bounds
                .left()
                .0
        };
        let frame = s
            .layout("id:rich-editor-table-scroll")
            .unwrap()
            .unwrap()
            .bounds;
        (left(first), left(last), frame.left().0, frame.right().0)
    };

    let short: &[&[&str]] = &[&["a1", "a2"], &["b1", "b2"]];
    let (a1, a2, _, right) = column_starts(false, short);
    assert!(
        a2 - a1 < 100.0,
        "columns fit their content by default: {a1} {a2}"
    );
    let (a1, a2, left, right_full) = column_starts(true, short);
    assert_eq!(right, right_full);
    let half = (right_full - left) / 2.0;
    assert!(
        (a2 - a1 - half).abs() < 2.0,
        "two columns share the width evenly: {a1} {a2} of {left}..{right_full}"
    );

    let prose = "a long paragraph of ordinary words that wraps inside its cell rather than widening the table";
    let (_, end, left, right) = column_starts(true, &[&[prose, "End"]]);
    assert!(
        end < right,
        "a long paragraph wraps in a full-width table: {end} {left}..{right}"
    );

    let wide_first = |s: &Screen| x_of(s, "s2") - x_of(s, "s1");

    // Every column sized: the table still spans the editor's text, the last
    // column taking the room left, and the first keeps its width.
    let (mut app, _) = full_width_app(sized_row(&[Some(100), Some(100)]));
    let s = app.screen();
    let (frame, edge) = (table_frame(s), table_edge(s));
    assert!(
        (wide_first(s) - 100.0).abs() < 1.0,
        "saved widths are kept: {}",
        wide_first(s)
    );
    assert!(
        (edge - frame.right().0).abs() < 4.0,
        "a fully sized table still spans the editor: edge {edge} of {frame:?}"
    );

    // Widening the first column takes the room from the last one.
    drag_band(&mut app, 0, 50.0);
    app.wait_until("the first column widens", Duration::from_secs(5), |s| {
        (wide_first(s) - 150.0).abs() < 1.0
    })
    .unwrap();
    assert!((table_edge(app.screen()) - frame.right().0).abs() < 4.0);

    // Sizing the last column without a width (the last resize) keeps the
    // table spanning the editor: it does not shrink to the columns' sum.
    let (mut app, held) = full_width_app(sized_row(&[Some(100), None]));
    let edge = table_edge(app.screen());
    assert!((edge - frame.right().0).abs() < 4.0, "edge {edge}");
    // What a resize of the last column runs when it is let go.
    let command = set_column_width(Some(0), 1, Some(px(60.0)));
    assert!(held.run(&command));
    app.wait_until("the last column has a width", Duration::from_secs(5), |_| {
        held.state().doc().to_json()["content"][0]["content"][0]["content"][1]["attrs"]
            ["colwidth"]
            == json!([60])
    })
    .unwrap();
    app.wait(Duration::from_millis(100)).unwrap();
    let edge = table_edge(app.screen());
    assert!(
        (edge - frame.right().0).abs() < 4.0,
        "the table still spans the editor after its last resize: edge {edge} of {frame:?}"
    );

    // The last column stretched past its width into the room left: dragging
    // it narrower shows nothing, so letting go saves nothing.
    let (mut app, held) = full_width_app(sized_row(&[Some(100), Some(100)]));
    let last_width = |held: &EditorSession| {
        held.state().doc().to_json()["content"][0]["content"][0]["content"][1]["attrs"]["colwidth"]
            [0]
        .as_f64()
        .unwrap()
    };
    let drawn = table_edge(app.screen()) - x_of(app.screen(), "s2");
    drag_band(&mut app, 1, -30.0);
    app.wait(Duration::from_millis(100)).unwrap();
    assert_eq!(last_width(&held), 100.0);
    // The same band, dragged wider (apart from the first press, so not a
    // double click), saves its drawn width and more: the press lands.
    app.wait(Duration::from_millis(500)).unwrap();
    drag_band(&mut app, 1, 30.0);
    app.wait_until("the last column widens", Duration::from_secs(5), |_| {
        last_width(&held) > f64::from(drawn) + 20.0
    })
    .unwrap();

    // Sized columns wider than the editor: the last column is as wide as
    // its width, so narrowing it narrows the table and is saved.
    let (mut app, held) = full_width_app(sized_row(&[Some(500), Some(500)]));
    app.scroll("id:rich-editor-table-scroll", px(10_000.0))
        .unwrap();
    app.wait(Duration::from_millis(100)).unwrap();
    drag_band(&mut app, 1, -50.0);
    app.wait_until("the last column narrows", Duration::from_secs(5), |_| {
        held.state().doc().to_json()["content"][0]["content"][0]["content"][1]["attrs"]["colwidth"]
            == json!([450])
    })
    .unwrap();

    // Sized columns wider than the editor: the table scrolls, and a column
    // past the editor's edge still resizes.
    let (mut app, held) = full_width_app(sized_row(&[Some(300), Some(300), Some(300)]));
    app.scroll("id:rich-editor-table-scroll", px(300.0))
        .unwrap();
    app.wait(Duration::from_millis(100)).unwrap();
    drag_band(&mut app, 1, 20.0);
    app.wait_until("the second column widens", Duration::from_secs(5), |_| {
        held.state().doc().to_json()["content"][0]["content"][0]["content"][1]["attrs"]["colwidth"]
            == json!([320])
    })
    .unwrap();

    let wide: &[&[&str]] = &[&[
        "ConferenceRegistrationConfirmation",
        "InternationalRelationsCoordinator",
        "AccessibleTransportationReservation",
        "End",
    ]];
    let (_, end, _, right) = column_starts(true, wide);
    assert!(
        end >= right,
        "a table wider than the editor still scrolls: {end} {right}"
    );
}

#[test]
fn a_tables_handles_follow_it_and_the_quote_it_is_in() {
    let mut config = EngineConfig {
        viewport: Size::new(px(800.0), px(900.0)),
        ..EngineConfig::new(nsh_testing::windowed_root_with::<Tables>(|| HarnessProps {
            session: Some(table_session()),
        }))
    };
    config.environment.os = Os::Linux;
    config.environment.idiom = Idiom::Desktop;
    let mut app = Headless::start(config).expect("the engine starts");
    see(&mut app, "Ready");
    // Two columns and two rows of the first table, two columns and a row
    // of the quoted one.
    app.wait_until("the handles", Duration::from_secs(5), |s| {
        bands(s).len() == 7
    })
    .unwrap();
    let before = bands(app.screen());
    let (table, quoted) = (top(app.screen(), "a1"), top(app.screen(), "q1"));

    app.press("id:grow").unwrap();
    see(&mut app, "Grown");
    app.wait_until("the tables move", Duration::from_secs(5), |s| {
        top(s, "a1") > table
    })
    .unwrap();
    let moved = top(app.screen(), "a1") - table;
    assert_eq!(
        top(app.screen(), "q1") - quoted,
        moved,
        "the quote moves with the first table"
    );
    app.wait_until("the handles move", Duration::from_secs(5), |s| {
        let after = bands(s);
        after.len() == before.len()
            && after.iter().zip(&before).all(|(a, b)| {
                a.size == b.size && a.origin.x == b.origin.x && a.origin.y.0 - b.origin.y.0 == moved
            })
    })
    .unwrap_or_else(|e| {
        panic!(
            "{e}\nbefore {before:?}\nafter {:?}\nmoved {moved}",
            bands(app.screen())
        )
    });
}

/// The column grips' bands, as laid out (24 x 12 on the active table's top
/// border).
fn col_grips(screen: &Screen) -> Vec<Bounds<Points>> {
    let mut out: Vec<Bounds<Points>> = screen
        .layout_nodes()
        .iter()
        .map(|n| n.bounds)
        .filter(|b| b.size.width.0 == 24.0 && b.size.height.0 == 12.0)
        .collect();
    out.sort_by(|a, b| a.origin.x.0.partial_cmp(&b.origin.x.0).unwrap());
    out
}

#[test]
fn a_table_grip_press_selects_with_the_pointer_meta() {
    let session = table_session();
    let noted = super::selection::note_pointer(&session);
    let held = session.clone();
    let mut config = EngineConfig {
        viewport: Size::new(px(800.0), px(900.0)),
        ..EngineConfig::new(nsh_testing::windowed_root_with::<Tables>(move || {
            HarnessProps {
                session: Some(session.clone()),
            }
        }))
    };
    config.environment.os = Os::Linux;
    config.environment.idiom = Idiom::Desktop;
    let mut app = Headless::start(config).expect("the engine starts");
    see(&mut app, "Ready");

    // Pressing a cell activates its table so its column and row grips show.
    app.press_at("text:a1", Point::new(px(1.0), px(1.0)))
        .unwrap();
    app.wait_until("the active column grip", Duration::from_secs(5), |s| {
        col_grips(s).len() == 1
    })
    .unwrap();
    assert_eq!(noted.lock().unwrap().last(), Some(&true));

    let grip = col_grips(app.screen())[0];
    let above = app
        .screen()
        .layout_nodes()
        .iter()
        .find(|n| n.text.as_ref().is_some_and(|t| t.text.text == "above"))
        .expect("the paragraph above the table")
        .bounds;
    let at = Point::new(
        grip.origin.x + grip.size.width * 0.5 - above.origin.x,
        grip.origin.y + grip.size.height * 0.5 - above.origin.y,
    );
    let before = noted.lock().unwrap().len();
    // Past the double-click window so the grip press lands as a single click.
    app.wait(Duration::from_millis(600)).unwrap();
    app.press_at("text:above", at).unwrap();
    app.wait_until("the column selection", Duration::from_secs(5), |_| {
        held.state().selection().json_id() == "cell"
    })
    .unwrap();
    let notes = noted.lock().unwrap();
    assert!(notes.len() > before);
    assert_eq!(notes.last(), Some(&true));
}

#[test]
fn table_grips_follow_the_hovered_or_pressed_cell_by_default() {
    for (os, idiom) in [(Os::Linux, Idiom::Desktop), (Os::Android, Idiom::Phone)] {
        let mut config = EngineConfig {
            viewport: Size::new(px(800.0), px(900.0)),
            ..EngineConfig::new(nsh_testing::windowed_root_with::<Tables>(|| HarnessProps {
                session: Some(table_session()),
            }))
        };
        config.environment.os = os;
        config.environment.idiom = idiom;
        let mut app = Headless::start(config).unwrap();
        see(&mut app, "Ready");
        let grips = |s: &Screen| -> Option<(f32, f32)> {
            let cols = col_grips(s);
            let rows: Vec<_> = s
                .layout_nodes()
                .into_iter()
                .map(|n| n.bounds)
                .filter(|b| b.size.width.0 == 12. && b.size.height.0 == 24.)
                .collect();
            match (cols.as_slice(), rows.as_slice()) {
                ([c], [r]) => Some((c.origin.x.0, r.origin.y.0)),
                _ => None,
            }
        };
        if idiom == Idiom::Desktop {
            app.hover("text:a1").unwrap();
        } else {
            app.press("text:a1").unwrap();
        }
        app.wait_until("one row and one column grip", Duration::from_secs(5), |s| {
            grips(s).is_some()
        })
        .unwrap();
        let first = grips(app.screen()).unwrap();
        if idiom == Idiom::Desktop {
            app.hover("text:b2").unwrap();
        } else {
            app.press("text:b2").unwrap();
        }
        app.wait_until(
            "the other row and column replace the previous grips",
            Duration::from_secs(5),
            |s| grips(s).is_some_and(|(x, y)| x > first.0 && y > first.1),
        )
        .unwrap();
        app.press("text:b2").unwrap();
        if idiom == Idiom::Desktop {
            app.hover("text:above").unwrap();
            assert!(grips(app.screen()).is_some_and(|(x, y)| x > first.0 && y > first.1));
        }
        app.press("text:above").unwrap();
        app.wait_until(
            "grips leave with the selection",
            Duration::from_secs(5),
            |s| col_grips(s).is_empty(),
        )
        .unwrap();
    }
}

/// A table whose header cells have a typed background (`StyleRefinement`,
/// as a Rust app types it) or the same in declarations.
#[derive(Clone, PartialEq, Props)]
struct HeaderProps {
    session: Option<EditorSession>,
    typed: bool,
}

#[component]
fn StyledHeader(props: HeaderProps) -> impl IntoElement {
    use nsh_base::rich_editor::view::{PartStyles, RichEditorStyles};
    use nsh_core::{PropertyValue, StyleKey, element::StyleRefinement};

    let red = rgb(0xff0000);
    let style = if props.typed {
        StyleRefinement::default().bg(red).into()
    } else {
        nsh_core::element::Style::default().with(
            StyleKey::BackgroundColor,
            PropertyValue::Color(Some(red.into())),
        )
    };
    let mut styles = RichEditorStyles::default();
    styles.nodes = Some(PartStyles([("tableHeader".to_owned(), style)].into()));
    view()
        .child(
            view().test_id("editor").w(px(600.0)).h(px(400.0)).child(
                rich_editor()
                    .editor(props.session.clone().expect("a session"))
                    .styles(styles)
                    .size_full(),
            ),
        )
        .child(text("Ready"))
}

/// A header cell's part style wins over the header's theme background,
/// typed as declared.
#[test]
fn a_header_cells_typed_part_style_wins_over_its_theme_background() {
    for typed in [false, true] {
        let mut config = EngineConfig {
            viewport: Size::new(px(800.0), px(900.0)),
            ..EngineConfig::new(nsh_testing::windowed_root_with::<StyledHeader>(move || {
                HeaderProps {
                    session: Some(session_with_content(json!([{"type": "table", "content": [
                        {"type": "tableRow", "content": [{"type": "tableHeader", "content": [
                            {"type": "paragraph", "content": [{"type": "text", "text": "Head"}]}
                        ]}]},
                        {"type": "tableRow", "content": [cell("body")]},
                    ]}]))),
                    typed,
                }
            }))
        };
        config.environment.os = Os::Linux;
        config.environment.idiom = Idiom::Desktop;
        let mut app = Headless::start(config).expect("the engine starts");
        see(&mut app, "Head");
        let screen = app.screen();
        let nodes = screen.layout_nodes();
        // The backgrounds of the views around the header's text.
        let mut at = screen.id("text:Head").unwrap();
        let mut backgrounds = Vec::new();
        while let Some(node) = nodes.iter().find(|n| n.id == at) {
            if let Ok(Some(style)) = screen.view_style(&format!("view:{at}")) {
                backgrounds.extend(style.background_color);
            }
            at = node.parent;
        }
        assert!(
            backgrounds.contains(&0xff00_00ff),
            "typed: {typed}, backgrounds: {backgrounds:x?}"
        );
    }
}

/// The editor and what its grips reported: the last press (its table, row,
/// column, grip box in the root and whether the press is in that box), and
/// how many.
#[component]
fn TableMenus(props: HarnessProps) -> impl IntoElement {
    let session = props.session.clone().expect("a session");
    let last = signal(String::from("Grip none"));
    let count = signal(0_u32);
    view()
        .child(view().test_id("editor").w(px(600.0)).h(px(600.0)).child(
            rich_editor().editor(session).size_full().on_table_menu(
                move |e: &RichEditorTableMenu| {
                    last.set(format!(
                        "Grip {} {:?} {:?} at {}",
                        e.table,
                        e.row,
                        e.column,
                        boxed(e.bounds)
                    ));
                    count.update(|c| *c += 1);
                },
            ),
        ))
        .child(text(move || last.get()))
        .child(text(move || format!("Presses {}", count.get())))
        .child(text("Ready"))
}

fn boxed(b: Bounds<Points>) -> String {
    format!(
        "{:.0},{:.0} {:.0}x{:.0}",
        b.origin.x.0, b.origin.y.0, b.size.width.0, b.size.height.0
    )
}

fn start_grips(session: EditorSession) -> Headless {
    start_grips_on(session, Os::Linux, Idiom::Desktop)
}

fn start_grips_on(session: EditorSession, os: Os, idiom: Idiom) -> Headless {
    let mut config = EngineConfig {
        viewport: Size::new(px(800.0), px(900.0)),
        ..EngineConfig::new(nsh_testing::windowed_root_with::<TableMenus>(move || {
            HarnessProps {
                session: Some(session.clone()),
            }
        }))
    };
    config.environment.os = os;
    config.environment.idiom = idiom;
    let mut app = Headless::start(config).expect("the engine starts");
    see(&mut app, "Ready");
    app
}

/// The active table's column grip (24 x 12) or row grip (12 x 24), as laid
/// out: its view and its box.
fn grip(screen: &Screen, column: bool) -> Option<(u32, Bounds<Points>)> {
    let (w, h) = if column { (24.0, 12.0) } else { (12.0, 24.0) };
    let found: Vec<_> = screen
        .layout_nodes()
        .into_iter()
        .filter(|n| n.bounds.size.width.0 == w && n.bounds.size.height.0 == h)
        .map(|n| (n.id, n.bounds))
        .collect();
    match found[..] {
        [one] => Some(one),
        _ => None,
    }
}

/// Puts the caret in the first table's first cell, so its grips show, and
/// waits out the double-click window so the next press is a single one.
fn activate_first_cell(app: &mut Headless) {
    app.press_at("text:a1", Point::new(px(1.0), px(1.0)))
        .unwrap();
    // The grips of that cell's column and row, not of an earlier selection.
    app.wait_until("the first cell's grips", Duration::from_secs(5), |s| {
        let Ok(Some(cell)) = s.layout("text:a1") else {
            return false;
        };
        let center = |b: Bounds<Points>| {
            (
                b.origin.x.0 + b.size.width.0 / 2.0,
                b.origin.y.0 + b.size.height.0 / 2.0,
            )
        };
        let (x, y) = center(cell.bounds);
        matches!(
            (grip(s, true), grip(s, false)),
            (Some((_, col)), Some((_, row)))
                if (center(col).0 - x).abs() < 12.0 && (center(row).1 - y).abs() < 12.0
        )
    })
    .unwrap();
    app.wait(Duration::from_millis(600)).unwrap();
}

#[test]
fn a_grip_pressed_without_dragging_reports_its_row_or_column() {
    let session = table_session();
    let held = session.clone();
    let mut app = start_grips(session);
    // The first table starts after the paragraph `above` (7 long).
    activate_first_cell(&mut app);
    let (id, b) = grip(app.screen(), true).unwrap();
    let center = Point::new(b.size.width * 0.5, b.size.height * 0.5);
    app.press_at(&format!("view:{id}"), center).unwrap();
    see(&mut app, &format!("Grip 7 None Some(0) at {}", boxed(b)));
    see(&mut app, "Presses 1");
    // The press still selects the column, as before it was reported.
    assert_eq!(held.state().selection().json_id(), "cell");

    activate_first_cell(&mut app);
    let (id, b) = grip(app.screen(), false).unwrap();
    let center = Point::new(b.size.width * 0.5, b.size.height * 0.5);
    app.press_at(&format!("view:{id}"), center).unwrap();
    see(&mut app, &format!("Grip 7 Some(0) None at {}", boxed(b)));
    see(&mut app, "Presses 2");
    assert_eq!(held.state().selection().json_id(), "cell");
}

#[test]
fn a_grip_dragged_past_the_slop_moves_its_column_without_a_press() {
    let session = table_session();
    let held = session.clone();
    let mut app = start_grips(session);
    activate_first_cell(&mut app);
    let (id, b) = grip(app.screen(), true).unwrap();
    let to = app.screen().layout("text:a2").unwrap().unwrap().bounds;
    // From the first column's grip to the middle of the second column.
    let by = Point::new(
        to.origin.x + to.size.width * 0.5 - (b.origin.x + b.size.width * 0.5),
        px(0.0),
    );
    app.drag(&format!("view:{id}"), by, DragOptions::default())
        .unwrap();
    app.wait_until("the column moves", Duration::from_secs(5), |_| {
        let doc = held.state().doc().to_json().to_string();
        doc.find("a2").unwrap() < doc.find("a1").unwrap()
    })
    .unwrap();
    app.wait(Duration::from_millis(100)).unwrap();
    see(&mut app, "Presses 0");
    see(&mut app, "Grip none");
}

#[test]
fn a_grip_wobbling_past_the_slop_across_its_line_neither_reports_nor_moves() {
    let session = table_session();
    let held = session.clone();
    let mut app = start_grips(session);
    activate_first_cell(&mut app);
    let before = held.state().doc().to_json().to_string();
    // 4.2 points away, but under 4 along the row: past the slop, too short to move the column.
    let (id, _) = grip(app.screen(), true).unwrap();
    app.drag(
        &format!("view:{id}"),
        Point::new(px(3.0), px(3.0)),
        DragOptions::default(),
    )
    .unwrap();
    app.wait(Duration::from_millis(200)).unwrap();
    see(&mut app, "Presses 0");
    assert_eq!(held.state().doc().to_json().to_string(), before);

    // Under the slop it is still a press.
    activate_first_cell(&mut app);
    let (id, _) = grip(app.screen(), true).unwrap();
    app.drag(
        &format!("view:{id}"),
        Point::new(px(2.0), px(2.0)),
        DragOptions::default(),
    )
    .unwrap();
    see(&mut app, "Presses 1");
    assert_eq!(held.state().doc().to_json().to_string(), before);
}

#[test]
fn a_tap_on_a_grip_reports_it_on_touch_screens() {
    let session = table_session();
    let mut app = start_grips_on(session, Os::Android, Idiom::Phone);
    app.press("text:a1").unwrap();
    app.wait_until("the first cell's grips", Duration::from_secs(5), |s| {
        grip(s, true).is_some()
    })
    .unwrap();
    app.wait(Duration::from_millis(600)).unwrap();
    let (id, b) = grip(app.screen(), true).unwrap();
    let center = Point::new(b.size.width * 0.5, b.size.height * 0.5);
    app.press_at(&format!("view:{id}"), center).unwrap();
    see(&mut app, &format!("Grip 7 None Some(0) at {}", boxed(b)));
    see(&mut app, "Presses 1");
}

#[test]
fn the_generated_js_event_passes_the_table_menu() {
    let mut registry = nsh_core::element::Registry::new();
    registry.register::<nsh_base::rich_editor::view::RichEditor>();
    let declarations = nsh_core::element::declarations(&registry, &[]);
    for wanted in [
        "onTableMenu?: (event: RichEditorTableMenu)",
        "export interface RichEditorTableMenu",
        "bounds: Bounds",
    ] {
        assert!(declarations.contains(wanted), "{wanted}\n{declarations}");
    }
}
