use owo_colors::{AnsiColors, OwoColorize};
use std::collections::BTreeMap;
use zellij_tile::prelude::*;
use zellij_tile::ui_components::{
    print_nested_list_with_coordinates, print_text_with_coordinates, NestedListItem, Text,
};

/// Interaction mode, vim-style.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Default: `j`/`k` move the selection, no typing into the filter.
    Normal,
    /// Filter mode: keystrokes are appended to the fuzzy filter.
    Insert,
}

/// How the plugin draws itself.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderStyle {
    /// Self-drawn with raw ANSI (owo-colors). Full control, fixed palette.
    Ansi,
    /// Zellij's native UI components — follows the active theme.
    Native,
}

/// Which list zhop is showing. `Tab` toggles between them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// Switch between the session's tabs (the original purpose).
    Tabs,
    /// Manage the floating panes on the current tab.
    Panes,
}

/// A floating pane on zhop's tab, as shown in the Panes view.
struct Pane {
    id: PaneId,
    title: String,
    /// Whether this pane is the focused one in the floating layer.
    focused: bool,
}

struct State {
    // ── tabs view ──
    tabs: Vec<TabInfo>,
    /// Position (0-based) of the highlighted tab.
    selected_tab: Option<usize>,

    // ── panes view ──
    /// Floating panes on zhop's tab (excluding zhop itself).
    panes: Vec<Pane>,
    /// Id of the highlighted pane.
    selected_pane: Option<PaneId>,

    view: View,
    filter: String,
    mode: Mode,

    // ── config ──
    ignore_case: bool,
    start_in_insert: bool,
    selection_color: AnsiColors,
    render_style: RenderStyle,
    /// Target width in columns. `LaunchOrFocusPlugin` can't size a plugin pane,
    /// so when set the plugin shrinks its own floating pane to ~this width.
    target_width: Option<usize>,

    /// Our own plugin pane id, so we can exclude ourselves from the pane list.
    own_plugin_id: u32,

    // ── auto-resize bookkeeping ──
    width_settled: bool,
    prev_cols: usize,
    resize_attempts: u16,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            selected_tab: None,
            panes: Vec::new(),
            selected_pane: None,
            view: View::Tabs,
            filter: String::new(),
            mode: Mode::Normal,
            ignore_case: true,
            start_in_insert: false,
            selection_color: AnsiColors::Yellow,
            render_style: RenderStyle::Ansi,
            target_width: None,
            own_plugin_id: 0,
            width_settled: false,
            prev_cols: 0,
            resize_attempts: 0,
        }
    }
}

/// Move a selection through `keys`, wrapping around. `cur` is the current key.
fn cycle<T: Copy + PartialEq>(keys: &[T], cur: Option<T>, down: bool) -> Option<T> {
    if keys.is_empty() {
        return None;
    }
    match cur.and_then(|c| keys.iter().position(|&k| k == c)) {
        Some(i) => {
            let n = keys.len();
            let j = if down { (i + 1) % n } else { (i + n - 1) % n };
            Some(keys[j])
        }
        None => Some(if down { keys[0] } else { *keys.last().unwrap() }),
    }
}

impl State {
    fn matches(&self, name: &str) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        if self.ignore_case {
            name.to_lowercase().contains(&self.filter.to_lowercase())
        } else {
            name.contains(&self.filter)
        }
    }

    fn viewable_tabs(&self) -> impl Iterator<Item = &TabInfo> {
        self.tabs.iter().filter(|t| self.matches(&t.name))
    }
    fn tab_keys(&self) -> Vec<usize> {
        self.viewable_tabs().map(|t| t.position).collect()
    }

    fn viewable_panes(&self) -> impl Iterator<Item = &Pane> {
        self.panes.iter().filter(|p| self.matches(&p.title))
    }
    fn pane_keys(&self) -> Vec<PaneId> {
        self.viewable_panes().map(|p| p.id).collect()
    }

    // ── selection (operates on whichever view is active) ──

    fn select_first(&mut self) {
        match self.view {
            View::Tabs => self.selected_tab = self.tab_keys().first().copied(),
            View::Panes => self.selected_pane = self.pane_keys().first().copied(),
        }
    }
    fn select_last(&mut self) {
        match self.view {
            View::Tabs => self.selected_tab = self.tab_keys().last().copied(),
            View::Panes => self.selected_pane = self.pane_keys().last().copied(),
        }
    }
    fn select_down(&mut self) {
        match self.view {
            View::Tabs => self.selected_tab = cycle(&self.tab_keys(), self.selected_tab, true),
            View::Panes => self.selected_pane = cycle(&self.pane_keys(), self.selected_pane, true),
        }
    }
    fn select_up(&mut self) {
        match self.view {
            View::Tabs => self.selected_tab = cycle(&self.tab_keys(), self.selected_tab, false),
            View::Panes => self.selected_pane = cycle(&self.pane_keys(), self.selected_pane, false),
        }
    }
    /// After the filter changes, snap the highlight to the first match.
    fn reset_selection(&mut self) {
        self.select_first();
    }

    /// Switch views (`Tab`), clearing the filter and fixing up the highlight.
    fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Tabs => View::Panes,
            View::Panes => View::Tabs,
        };
        self.filter.clear();
        match self.view {
            View::Tabs => {
                let keys = self.tab_keys();
                if self.selected_tab.map_or(true, |s| !keys.contains(&s)) {
                    self.selected_tab = keys.first().copied();
                }
            }
            View::Panes => {
                let keys = self.pane_keys();
                if self.selected_pane.map_or(true, |s| !keys.contains(&s)) {
                    self.selected_pane = keys.first().copied();
                }
            }
        }
    }

    /// Rebuild the floating-pane list for zhop's tab from a pane manifest.
    fn refresh_panes(&mut self, manifest: &PaneManifest) {
        let our_tab = manifest.panes.iter().find_map(|(tab, panes)| {
            panes
                .iter()
                .any(|p| p.is_plugin && p.id == self.own_plugin_id)
                .then_some(*tab)
        });
        self.panes = our_tab
            .and_then(|t| manifest.panes.get(&t))
            .map(|panes| {
                panes
                    .iter()
                    // visible floating panes, never ourselves (focus/close only
                    // work on panes that are actually in the layer)
                    .filter(|p| {
                        p.is_floating
                            && !p.is_suppressed
                            && !(p.is_plugin && p.id == self.own_plugin_id)
                    })
                    .map(|p| Pane {
                        id: if p.is_plugin {
                            PaneId::Plugin(p.id)
                        } else {
                            PaneId::Terminal(p.id)
                        },
                        title: if p.title.trim().is_empty() {
                            format!("(pane {})", p.id)
                        } else {
                            p.title.clone()
                        },
                        focused: p.is_focused,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // keep the highlight pointing at a pane that still exists
        let keys = self.pane_keys();
        if self.selected_pane.map_or(true, |s| !keys.contains(&s)) {
            self.selected_pane = keys.first().copied();
        }
    }

    /// Act on the highlighted row: switch tab (Tabs) or focus pane (Panes), then close.
    fn confirm(&self) {
        match self.view {
            View::Tabs => {
                if let Some(tab) = self
                    .tabs
                    .iter()
                    .find(|tab| Some(tab.position) == self.selected_tab)
                {
                    close_focus();
                    // Zellij's switch_tab_to is 1-based; TabInfo.position is 0-based.
                    switch_tab_to(tab.position as u32 + 1);
                }
            }
            View::Panes => {
                if let Some(id) = self.selected_pane {
                    close_focus();
                    focus_pane_with_id(id, true);
                }
            }
        }
    }

    /// Close the highlighted floating pane (Panes view). The list refreshes on
    /// the resulting `PaneUpdate`.
    fn close_selected_pane(&self) {
        if let Some(id) = self.selected_pane {
            close_pane_with_id(id);
        }
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, mut configuration: BTreeMap<String, String>) {
        // ReadApplicationState  → receive TabUpdate / PaneUpdate / Key events
        // ChangeApplicationState → switch tabs, focus/close panes, close ourselves
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        self.own_plugin_id = get_plugin_ids().plugin_id;

        if let Some(v) = configuration.remove("ignore_case") {
            self.ignore_case = v.trim().parse().unwrap_or(true);
        }
        if let Some(v) = configuration.remove("start_in_insert") {
            self.start_in_insert = v.trim().parse().unwrap_or(false);
        }
        if let Some(c) = configuration.remove("selection_color") {
            self.selection_color = c.trim().into();
        }
        if let Some(v) = configuration.remove("ui") {
            self.render_style = match v.trim().to_lowercase().as_str() {
                "native" => RenderStyle::Native,
                _ => RenderStyle::Ansi,
            };
        }
        if let Some(v) = configuration.remove("width") {
            self.target_width = v.trim().parse::<usize>().ok().filter(|w| *w > 0);
        }

        self.mode = if self.start_in_insert {
            Mode::Insert
        } else {
            Mode::Normal
        };

        subscribe(&[EventType::TabUpdate, EventType::PaneUpdate, EventType::Key]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                // Default the highlight to the currently active tab.
                if self.selected_tab.is_none() {
                    self.selected_tab = tabs.iter().find_map(|t| t.active.then_some(t.position));
                }
                self.tabs = tabs;
                true
            }
            Event::PaneUpdate(manifest) => {
                self.refresh_panes(&manifest);
                // only the Panes view depends on this
                matches!(self.view, View::Panes)
            }
            Event::Key(key) => match self.mode {
                Mode::Normal => self.handle_normal_key(key),
                Mode::Insert => self.handle_insert_key(key),
            },
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        match self.render_style {
            RenderStyle::Ansi => self.render_ansi(rows, cols),
            RenderStyle::Native => self.render_native(rows, cols),
        }
        self.autoresize_width(cols);
    }
}

impl State {
    /// Work around `LaunchOrFocusPlugin` not accepting pane dimensions: when a
    /// `target_width` is configured, nudge our own floating pane narrower (one
    /// relative resize step per frame) until we reach it. Each resize triggers
    /// a re-render, so this converges over a few frames and then settles.
    fn autoresize_width(&mut self, cols: usize) {
        let Some(target) = self.target_width else {
            return;
        };
        if self.width_settled {
            return;
        }
        // reached the target → done
        if cols <= target {
            self.width_settled = true;
            return;
        }
        // last resize didn't shrink us (hit a minimum) → stop trying
        if self.prev_cols != 0 && cols >= self.prev_cols {
            self.width_settled = true;
            return;
        }
        // safety cap against an unexpected resize loop
        if self.resize_attempts >= 100 {
            self.width_settled = true;
            return;
        }
        self.prev_cols = cols;
        self.resize_attempts += 1;
        resize_focused_pane_with_direction(Resize::Decrease, Direction::Right);
    }
}

/// One rendered row, independent of the renderer.
struct Row {
    label: String,
    selected: bool,
    /// active tab / focused pane — emphasized.
    marked: bool,
}

impl State {
    fn current_rows(&self) -> Vec<Row> {
        match self.view {
            View::Tabs => self
                .viewable_tabs()
                .map(|t| Row {
                    label: format!("{} {}", t.position + 1, t.name),
                    selected: self.selected_tab == Some(t.position),
                    marked: t.active,
                })
                .collect(),
            View::Panes => self
                .viewable_panes()
                .map(|p| Row {
                    label: p.title.clone(),
                    selected: self.selected_pane == Some(p.id),
                    marked: p.focused,
                })
                .collect(),
        }
    }

    fn empty_label(&self) -> &'static str {
        match self.view {
            View::Tabs => "no matching tabs",
            View::Panes if self.filter.is_empty() => "no floating panes on this tab",
            View::Panes => "no matching panes",
        }
    }

    fn hint(&self) -> &'static str {
        match (self.mode, self.view) {
            (Mode::Insert, _) => "type to filter · esc normal · enter select",
            (Mode::Normal, View::Tabs) => "j/k move · / filter · enter open · tab→panes · q quit",
            (Mode::Normal, View::Panes) => "j/k move · enter focus · x close · tab→tabs · q quit",
        }
    }

    /// Self-drawn renderer using raw ANSI escapes (owo-colors).
    fn render_ansi(&self, _rows: usize, _cols: usize) {
        // line 1: the two views as a tab strip — the active one is bracketed and colored.
        let tab = |text: &str, active: bool| {
            if active {
                format!("[{}]", text)
                    .color(self.selection_color)
                    .bold()
                    .to_string()
            } else {
                text.to_string()
            }
        };
        println!(
            "{}   {}",
            tab("TABS", self.view == View::Tabs),
            tab("PANES", self.view == View::Panes),
        );

        // line 2: mode badge + filter
        let badge = match self.mode {
            Mode::Normal => " NORMAL ".black().on_cyan().bold().to_string(),
            Mode::Insert => " INSERT ".black().on_yellow().bold().to_string(),
        };
        let filter = if self.filter.is_empty() {
            "(no filter)".dimmed().italic().to_string()
        } else {
            self.filter.clone()
        };
        println!("{} {} {}", badge, ">".cyan().bold(), filter);
        println!();

        let rows: Vec<String> = self
            .current_rows()
            .iter()
            .map(|r| {
                let pointer = if r.selected {
                    "›".color(self.selection_color).bold().to_string()
                } else {
                    " ".to_string()
                };
                let mut label = r.label.clone();
                if r.marked {
                    label = label.underline().to_string();
                }
                let label = if r.selected {
                    label.color(self.selection_color).bold().to_string()
                } else {
                    label
                };
                format!("{} {}", pointer, label)
            })
            .collect();

        if rows.is_empty() {
            println!("{}", format!("  {}", self.empty_label()).dimmed().italic());
        } else {
            println!("{}", rows.join("\n"));
        }

        println!();
        println!("{}", self.hint().dimmed());
    }

    /// Renderer using Zellij's native UI components — follows the active theme.
    fn render_native(&self, _rows: usize, _cols: usize) {
        // row 0: the two views as a tab strip — the active one is bracketed and colored.
        const TABS: &str = "TABS";
        const SEP: &str = "   ";
        const PANES: &str = "PANES";
        let (strip, range) = match self.view {
            View::Tabs => {
                let active = format!("[{}]", TABS);
                let len = active.chars().count();
                (format!("{}{}{}", active, SEP, PANES), 0..len)
            }
            View::Panes => {
                let active = format!("[{}]", PANES);
                let start = TABS.chars().count() + SEP.chars().count();
                let len = active.chars().count();
                (format!("{}{}{}", TABS, SEP, active), start..(start + len))
            }
        };
        let strip = Text::new(strip).color_range(2, range);
        print_text_with_coordinates(strip, 0, 0, None, None);

        // row 1: mode + filter
        let mode = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        };
        let filter = if self.filter.is_empty() {
            "(no filter)".to_string()
        } else {
            self.filter.clone()
        };
        print_text_with_coordinates(Text::new(format!("{}  {}", mode, filter)), 0, 1, None, None);

        // row 3: the list (row 2 left blank)
        let rows = self.current_rows();
        if rows.is_empty() {
            print_text_with_coordinates(Text::new(self.empty_label()), 0, 3, None, None);
            return;
        }
        let items: Vec<NestedListItem> = rows
            .iter()
            .map(|r| {
                let mut item = NestedListItem::new(r.label.clone());
                if r.marked {
                    item = item.color_range(0, ..);
                }
                if r.selected {
                    item = item.selected();
                }
                item
            })
            .collect();
        let count = items.len();
        print_nested_list_with_coordinates(items, 0, 3, None, None);
        print_text_with_coordinates(Text::new(self.hint()), 0, 3 + count + 1, None, None);
    }
}

impl State {
    fn handle_normal_key(&mut self, key: KeyWithModifier) -> bool {
        match key.bare_key {
            BareKey::Esc => {
                close_focus();
                false
            }
            BareKey::Char('q') if key.has_no_modifiers() => {
                close_focus();
                false
            }
            BareKey::Char('c') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                close_focus();
                false
            }
            BareKey::Enter => {
                self.confirm();
                false
            }

            // toggle Tabs ↔ Panes
            BareKey::Tab => {
                self.toggle_view();
                true
            }

            // close the highlighted floating pane (Panes view only)
            BareKey::Char('x') if key.has_no_modifiers() && self.view == View::Panes => {
                self.close_selected_pane();
                true
            }

            // enter filter mode
            BareKey::Char('/') if key.has_no_modifiers() => {
                self.mode = Mode::Insert;
                true
            }
            BareKey::Char('i') if key.has_no_modifiers() => {
                self.mode = Mode::Insert;
                true
            }

            // movement
            BareKey::Char('j') if key.has_no_modifiers() => {
                self.select_down();
                true
            }
            BareKey::Down => {
                self.select_down();
                true
            }
            BareKey::Char('n') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.select_down();
                true
            }
            BareKey::Char('k') if key.has_no_modifiers() => {
                self.select_up();
                true
            }
            BareKey::Up => {
                self.select_up();
                true
            }
            BareKey::Char('p') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.select_up();
                true
            }

            // top / bottom
            BareKey::Char('g') if key.has_no_modifiers() => {
                self.select_first();
                true
            }
            BareKey::Char('G') => {
                self.select_last();
                true
            }
            BareKey::Char('g') if key.has_modifiers(&[KeyModifier::Shift]) => {
                self.select_last();
                true
            }

            _ => false,
        }
    }

    fn handle_insert_key(&mut self, key: KeyWithModifier) -> bool {
        match key.bare_key {
            BareKey::Esc => {
                // leave filter mode but keep the typed filter
                self.mode = Mode::Normal;
                true
            }
            BareKey::Char('c') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                close_focus();
                false
            }
            BareKey::Enter => {
                self.confirm();
                false
            }
            BareKey::Backspace => {
                self.filter.pop();
                self.reset_selection();
                true
            }
            BareKey::Char('u') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.filter.clear();
                self.reset_selection();
                true
            }

            // navigation also works while filtering
            BareKey::Down => {
                self.select_down();
                true
            }
            BareKey::Char('n') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.select_down();
                true
            }
            BareKey::Up => {
                self.select_up();
                true
            }
            BareKey::Char('k') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.select_up();
                true
            }
            BareKey::Char('p') if key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.select_up();
                true
            }

            // plain text → filter
            BareKey::Char(c) if !key.has_modifiers(&[KeyModifier::Ctrl]) => {
                self.filter.push(c);
                self.reset_selection();
                true
            }

            _ => false,
        }
    }
}
