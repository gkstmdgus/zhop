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

struct State {
    tabs: Vec<TabInfo>,
    filter: String,
    /// Position (0-based) of the currently highlighted tab.
    selected: Option<usize>,
    mode: Mode,

    // ── config ──
    ignore_case: bool,
    start_in_insert: bool,
    selection_color: AnsiColors,
    render_style: RenderStyle,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            filter: String::new(),
            selected: None,
            mode: Mode::Normal,
            ignore_case: true,
            start_in_insert: false,
            selection_color: AnsiColors::Yellow,
            render_style: RenderStyle::Ansi,
        }
    }
}

impl State {
    fn matches_filter(&self, tab: &&TabInfo) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        if self.ignore_case {
            tab.name
                .to_lowercase()
                .contains(&self.filter.to_lowercase())
        } else {
            tab.name.contains(&self.filter)
        }
    }

    fn viewable_tabs_iter(&self) -> impl Iterator<Item = &TabInfo> {
        self.tabs.iter().filter(|tab| self.matches_filter(tab))
    }

    fn viewable_positions(&self) -> Vec<usize> {
        self.viewable_tabs_iter().map(|tab| tab.position).collect()
    }

    /// Highlight the first viewable tab (used after the filter changes).
    fn reset_selection(&mut self) {
        self.selected = self.viewable_positions().first().copied();
    }

    fn select_first(&mut self) {
        self.selected = self.viewable_positions().first().copied();
    }

    fn select_last(&mut self) {
        self.selected = self.viewable_positions().last().copied();
    }

    fn select_down(&mut self) {
        let positions = self.viewable_positions();
        if positions.is_empty() {
            self.selected = None;
            return;
        }
        match self.selected.and_then(|s| positions.iter().position(|&p| p == s)) {
            // wrap around to the top after the last entry
            Some(idx) => self.selected = Some(positions[(idx + 1) % positions.len()]),
            None => self.selected = Some(positions[0]),
        }
    }

    fn select_up(&mut self) {
        let positions = self.viewable_positions();
        if positions.is_empty() {
            self.selected = None;
            return;
        }
        match self.selected.and_then(|s| positions.iter().position(|&p| p == s)) {
            // wrap around to the bottom before the first entry
            Some(idx) => {
                let len = positions.len();
                self.selected = Some(positions[(idx + len - 1) % len]);
            }
            None => self.selected = Some(*positions.last().unwrap()),
        }
    }

    /// Switch to the highlighted tab and close the plugin.
    fn confirm(&self) {
        if let Some(tab) = self
            .tabs
            .iter()
            .find(|tab| Some(tab.position) == self.selected)
        {
            close_focus();
            // Zellij's switch_tab_to is 1-based; TabInfo.position is 0-based.
            switch_tab_to(tab.position as u32 + 1);
        }
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, mut configuration: BTreeMap<String, String>) {
        // ReadApplicationState  → receive TabUpdate / Key events
        // ChangeApplicationState → switch tabs, close the floating pane
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

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

        self.mode = if self.start_in_insert {
            Mode::Insert
        } else {
            Mode::Normal
        };

        subscribe(&[EventType::TabUpdate, EventType::Key]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                // Default the highlight to the currently active tab.
                if self.selected.is_none() {
                    self.selected = tabs
                        .iter()
                        .find_map(|t| t.active.then_some(t.position));
                }
                self.tabs = tabs;
                true
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
    }
}

impl State {
    /// Self-drawn renderer using raw ANSI escapes (owo-colors).
    fn render_ansi(&self, _rows: usize, _cols: usize) {
        let (badge, hint) = match self.mode {
            Mode::Normal => (
                " NORMAL ".black().on_cyan().bold().to_string(),
                "j/k move · / filter · enter open · q quit".dimmed().to_string(),
            ),
            Mode::Insert => (
                " INSERT ".black().on_yellow().bold().to_string(),
                "type to filter · esc normal · enter open".dimmed().to_string(),
            ),
        };

        let filter = if self.filter.is_empty() {
            "(no filter)".dimmed().italic().to_string()
        } else {
            self.filter.clone()
        };
        println!("{} {} {}", badge, ">".cyan().bold(), filter);
        println!();

        let rows: Vec<String> = self
            .viewable_tabs_iter()
            .map(|tab| {
                let is_selected = self.selected == Some(tab.position);
                let pointer = if is_selected {
                    "›".color(self.selection_color).bold().to_string()
                } else {
                    " ".to_string()
                };
                let mut name = tab.name.clone();
                if tab.active {
                    name = name.underline().to_string();
                }
                let label = format!("{} {}", tab.position + 1, name);
                let label = if is_selected {
                    label.color(self.selection_color).bold().to_string()
                } else {
                    label
                };
                format!("{} {}", pointer, label)
            })
            .collect();

        if rows.is_empty() {
            println!("{}", "  no matching tabs".dimmed().italic());
        } else {
            println!("{}", rows.join("\n"));
        }

        println!();
        println!("{}", hint);
    }

    /// Renderer using Zellij's native UI components — follows the active theme.
    fn render_native(&self, _rows: usize, _cols: usize) {
        // header: mode word (themed) + current filter
        let mode = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        };
        let filter = if self.filter.is_empty() {
            "(no filter)".to_string()
        } else {
            self.filter.clone()
        };
        let header = format!("{}  {}", mode, filter);
        let header = Text::new(header).color_range(2, 0..mode.chars().count());
        print_text_with_coordinates(header, 0, 0, None, None);

        // tab list — `.selected()` and active coloring use the theme palette
        let items: Vec<NestedListItem> = self
            .viewable_tabs_iter()
            .map(|tab| {
                let label = format!("{}  {}", tab.position + 1, tab.name);
                let mut item = NestedListItem::new(label);
                if tab.active {
                    item = item.color_range(0, ..);
                }
                if self.selected == Some(tab.position) {
                    item = item.selected();
                }
                item
            })
            .collect();

        if items.is_empty() {
            print_text_with_coordinates(Text::new("no matching tabs"), 0, 2, None, None);
            return;
        }
        let count = items.len();
        print_nested_list_with_coordinates(items, 0, 2, None, None);

        let hint = match self.mode {
            Mode::Normal => "j/k move · / filter · enter open · q quit",
            Mode::Insert => "type to filter · esc normal · enter open",
        };
        print_text_with_coordinates(Text::new(hint), 0, 2 + count + 1, None, None);
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
