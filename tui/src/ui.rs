use std::sync::{atomic::AtomicUsize, Arc};

use crossterm::event::KeyCode;
use ratatui::{prelude::*, widgets::*};

use crate::{
    events::{EventProxy, EventSender, Events},
    state::State,
    util::{EasyAtomic, SOFTWARE},
};

#[derive(Default)]
struct Context {
    tab_index: AtomicUsize,
    get_report_index: AtomicUsize,
}

pub struct Ui {
    layout: Layout,
    ctx: Arc<Context>,
    eventer: EventSender,
    content: ContentWidget,
}

impl Ui {
    pub fn new(event_proxy: EventProxy, state: Arc<State>) -> Self {
        let ctx = Arc::new(Context::default());
        Self {
            ctx: ctx.clone(),
            eventer: event_proxy.get_sender(),
            content: ContentWidget::new(ctx, state),
            layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(100)]),
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = self.layout.split(frame.size())[0];
        self.content.draw(frame, area);

        frame.render_widget(Block::default().title(SOFTWARE).borders(Borders::ALL), area);
    }

    pub fn input(&self, key: KeyCode) {
        match key {
            KeyCode::Right => {
                self.ctx.tab_index.set(1);
                self.eventer.send(Events::StopGetReport);
            }
            KeyCode::Left => {
                self.ctx.tab_index.set(0);
                self.eventer.send(Events::StartGetReport);
                self.eventer.send(Events::GetReport(0));
            }
            KeyCode::Char('p') => {
                let index = self.ctx.get_report_index.get();
                self.ctx.get_report_index.set(index + 1);
                self.eventer.send(Events::GetReport(index as u32 + 1));
            }
            KeyCode::Char('n') => {
                let index = self.ctx.get_report_index.get();
                if index > 0 {
                    self.ctx.get_report_index.set(index - 1);
                    self.eventer.send(Events::GetReport(index as u32 - 1));
                }
            }
            _ => (),
        }
    }
}

struct ContentWidget {
    layout: Layout,
    body: BodyWidget,
}

impl ContentWidget {
    fn new(ctx: Arc<Context>, state: Arc<State>) -> Self {
        Self {
            body: BodyWidget::new(ctx, state),
            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Percentage(95), Constraint::Percentage(5)])
                .margin(1),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let rects = self.layout.split(area);
        self.body.draw(frame, rects[0]);

        frame.render_widget(
            Paragraph::new("Help: Q - exit, ↑|←|↓|→ - select, P - previous page, N - next page, Enter - selected, Esc - go back")
                .alignment(Alignment::Left),
            rects[1],
        );
    }
}

struct BodyWidget {
    layout: Layout,
    sidebar: SidebarWidget,
    showcase: ShowcaseWidget,
}

impl BodyWidget {
    fn new(ctx: Arc<Context>, state: Arc<State>) -> Self {
        Self {
            sidebar: SidebarWidget::new(state.clone()),
            showcase: ShowcaseWidget::new(ctx, state),
            layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)]),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let rects = self.layout.split(area);
        self.sidebar.draw(frame, rects[0]);
        self.showcase.draw(frame, rects[1]);
    }
}

struct SidebarWidget {
    layout: Layout,
    stats: StatsWidget,
    interfaces: InterfacesWidget,
}

impl SidebarWidget {
    fn new(state: Arc<State>) -> Self {
        Self {
            stats: StatsWidget::new(state.clone()),
            interfaces: InterfacesWidget::new(state),
            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(7), Constraint::Percentage(70)]),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let rects = self.layout.split(area);
        self.stats.draw(frame, rects[0]);
        self.interfaces.draw(frame, rects[1]);
    }
}

struct StatsWidget {
    state: Arc<State>,
}

impl StatsWidget {
    fn new(state: Arc<State>) -> Self {
        Self { state }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let stats = self.state.get_stats();
        frame.render_widget(
            Paragraph::new(
                [
                    ("software", stats.software.clone()),
                    ("realm", stats.realm.clone()),
                    ("uptime", format!("{} - minute", stats.uptime / 60)),
                    ("capacity", stats.port_capacity.to_string()),
                    ("allocated", stats.port_allocated.to_string()),
                ]
                .into_iter()
                .map(|(k, v)| Line::from(vec![Span::raw(k).blue(), ": ".into(), Span::raw(v)]))
                .collect::<Vec<Line>>(),
            )
            .block(Block::default().title("Stats").borders(Borders::ALL))
            .alignment(Alignment::Left),
            area,
        );
    }
}

struct InterfacesWidget {
    state: Arc<State>,
}

impl InterfacesWidget {
    fn new(state: Arc<State>) -> Self {
        Self { state }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let stats = self.state.get_stats();
        frame.render_widget(
            Table::new(
                stats.interfaces.iter().map(|item| {
                    Row::new(vec![
                        if item.transport == 0 { "tcp" } else { "udp" },
                        item.bind.as_ref(),
                        item.external.as_ref(),
                    ])
                }),
                [
                    Constraint::Percentage(20),
                    Constraint::Percentage(40),
                    Constraint::Percentage(40),
                ],
            )
            .column_spacing(1)
            .header(
                Row::new(vec!["transport", "bind", "external"])
                    .style(Style::new().bg(Color::DarkGray).fg(Color::White)),
            )
            .block(Block::default().title("Interfaces").borders(Borders::ALL)),
            area,
        );
    }
}

struct ShowcaseWidget {
    layout: Layout,
    tabs: TabsWidget,
    tables: TablesWidget,
}

impl ShowcaseWidget {
    fn new(ctx: Arc<Context>, state: Arc<State>) -> Self {
        Self {
            tabs: TabsWidget::new(ctx.clone()),
            tables: TablesWidget::new(ctx, state),
            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(3), Constraint::Percentage(95)]),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let rects = self.layout.split(area);
        self.tabs.draw(frame, rects[0]);
        self.tables.draw(frame, rects[1]);
    }
}

struct TabsWidget {
    ctx: Arc<Context>,
}

impl TabsWidget {
    fn new(ctx: Arc<Context>) -> Self {
        Self { ctx }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Tabs::new(vec!["report", "users"])
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().white())
                .highlight_style(Style::default().yellow())
                .select(self.ctx.tab_index.get()),
            area,
        );
    }
}

struct TablesWidget {
    state: Arc<State>,
    ctx: Arc<Context>,
}

impl TablesWidget {
    fn new(ctx: Arc<Context>, state: Arc<State>) -> Self {
        Self { ctx, state }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let reports = self.state.get_report();
        frame.render_widget(
            Table::new(
                reports.iter().map(|item| {
                    Row::new(vec![
                        item.addr.clone(),
                        item.received_bytes.to_string(),
                        item.send_bytes.to_string(),
                        item.received_pkts.to_string(),
                        item.send_pkts.to_string(),
                    ])
                }),
                [
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ],
            )
            .column_spacing(1)
            .header(
                Row::new(vec![
                    "socket addrs",
                    "received bytes",
                    "send bytes",
                    "received packages",
                    "send packages",
                ])
                .style(Style::new().bg(Color::DarkGray).fg(Color::White)),
            )
            .block(
                Block::default()
                    .title(if self.ctx.tab_index.get() == 0 {
                        "Reports"
                    } else {
                        "Users"
                    })
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::new().reversed())
            .highlight_symbol("*"),
            area,
        );
    }
}
