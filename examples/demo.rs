// examples/demo.rs
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ttui::app::{run, App};
use ttui::buffer::LayerStack;
use ttui::layout::{Constraint, Direction, Layout, Rect};
use ttui::widgets::{block::Block, list::List, table::Table, text::Text};

#[derive(PartialEq, Clone, Copy)]
enum Focus {
    List,
    Table,
}

struct Demo {
    list_items: Vec<String>,
    list_selected: usize,
    table_headers: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_selected: usize,
    focus: Focus,
    quit: bool,
}

impl Demo {
    fn new() -> Self {
        Demo {
            list_items: vec!["Alpha".into(), "Beta".into(), "Gamma".into()],
            list_selected: 0,
            table_headers: vec!["Name".into(), "Status".into()],
            table_rows: vec![
                vec!["svc-a".into(), "ok".into()],
                vec!["svc-b".into(), "down".into()],
            ],
            table_selected: 0,
            focus: Focus::List,
            quit: false,
        }
    }
}

impl App for Demo {
    fn update(&mut self, event: &Event) {
        let Event::Key(k) = event else { return };
        if k.kind != KeyEventKind::Press {
            return;
        }
        match k.code {
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::List => Focus::Table,
                    Focus::Table => Focus::List,
                };
            }
            KeyCode::Down => match self.focus {
                Focus::List => {
                    self.list_selected = (self.list_selected + 1).min(self.list_items.len() - 1)
                }
                Focus::Table => {
                    self.table_selected = (self.table_selected + 1).min(self.table_rows.len() - 1)
                }
            },
            KeyCode::Up => match self.focus {
                Focus::List => self.list_selected = self.list_selected.saturating_sub(1),
                Focus::Table => self.table_selected = self.table_selected.saturating_sub(1),
            },
            KeyCode::Char('q') => self.quit = true,
            _ => {}
        }
    }

    fn view(&self, area: Rect, buf: &mut LayerStack) {
        let rows = Layout::new(
            Direction::Vertical,
            vec![Constraint::Fill(1), Constraint::Fixed(1)],
        )
        .split(area);
        let cols = Layout::new(
            Direction::Horizontal,
            vec![Constraint::Percentage(40), Constraint::Fill(1)],
        )
        .split(rows[0]);

        let list_inner = Block::new().title("Items").render(cols[0], buf);
        List::new(&self.list_items, self.list_selected).render(list_inner, buf);

        let table_inner = Block::new().title("Services").render(cols[1], buf);
        Table::new(
            &self.table_headers,
            &self.table_rows,
            self.table_selected,
            8,
        )
        .render(table_inner, buf);

        Text::new("Tab: switch focus | Up/Down: navigate | q: quit").render(rows[1], buf);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    let mut demo = Demo::new();
    run(&mut demo)
}
