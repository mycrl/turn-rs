use std::{
    net::SocketAddr,
    sync::{atomic::AtomicUsize, Arc, RwLock},
};

use crossterm::event::KeyCode;
use ratatui::{layout::SegmentSize, prelude::*, widgets::*};
use turn_drive::controller::Report;

use crate::{
    events::{EventProxy, EventSender, Events},
    state::State,
    util::{EasyAtomic, SOFTWARE},
};

#[derive(Default)]
struct Context {
    index: AtomicUsize,
    get_users_index: AtomicUsize,
    addrs: RwLock<Vec<SocketAddr>>,
}

pub struct Ui {
    layout: Layout,
    ctx: Arc<Context>,
    eventer: EventSender,
    content: ContentWidget,
    state: Arc<State>,
}

impl Ui {
    pub fn new(event_proxy: EventProxy, state: Arc<State>) -> Self {
        let ctx = Arc::new(Context::default());
        Self {
            ctx: ctx.clone(),
            state: state.clone(),
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
            KeyCode::Esc => {
                self.eventer.send(Events::ClearSession);
            }
            KeyCode::Delete => {
                if let Some(addr) = self
                    .ctx
                    .addrs
                    .read()
                    .unwrap()
                    .get(self.ctx.index.get())
                    .cloned()
                {
                    self.eventer.send(Events::RemoveSession(addr));
                }
            }
            KeyCode::Enter => {
                if let Some(addr) = self
                    .ctx
                    .addrs
                    .read()
                    .unwrap()
                    .get(self.ctx.index.get())
                    .cloned()
                {
                    self.eventer.send(Events::GetSession(addr));
                }
            }
            KeyCode::Down => {
                if self.state.get_session().is_none() {
                    let size = self.state.get_users().len();
                    let index = self.ctx.index.get();
                    if index + 1 < size {
                        self.ctx.index.set(index + 1);
                    }
                }
            }
            KeyCode::Up => {
                if self.state.get_session().is_none() {
                    let index = self.ctx.index.get();
                    if index > 0 {
                        self.ctx.index.set(index - 1);
                    }
                }
            }
            KeyCode::Char('n') => {
                let size = self.state.get_users().len();
                let index = self.ctx.get_users_index.get();
                if size >= 20 {
                    self.ctx.get_users_index.set(index + 1);
                    self.eventer.send(Events::GetUsers(index as u32 + 1));
                }
            }
            KeyCode::Char('p') => {
                let index = self.ctx.get_users_index.get();
                if index > 0 {
                    self.ctx.get_users_index.set(index - 1);
                    self.eventer.send(Events::GetUsers(index as u32 - 1));
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
            Paragraph::new(
                "Help: Q - exit, ↑|←|↓|→ - select, P - previous page, N - next page, \
                Enter - selected, Esc - go back, Delete - remove session.",
            )
            .alignment(Alignment::Left),
            rects[1],
        );
    }
}

struct BodyWidget {
    layout: Layout,
    sidebar: SidebarWidget,
    tables: TablesWidget,
}

impl BodyWidget {
    fn new(ctx: Arc<Context>, state: Arc<State>) -> Self {
        Self {
            sidebar: SidebarWidget::new(state.clone()),
            tables: TablesWidget::new(ctx, state),
            layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)]),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let rects = self.layout.split(area);
        self.sidebar.draw(frame, rects[0]);
        self.tables.draw(frame, rects[1]);
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
                    ("uptime", Self::time_str(stats.uptime)),
                    ("capacity", stats.capacity.to_string()),
                    ("allocated", stats.allocated.to_string()),
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

    fn time_str(seconds: u64) -> String {
        let mut time = seconds as f64;
        let mut date = (0.0, 0.0, 0.0);

        loop {
            if time < 60.0 {
                date.2 = time;
                break;
            } else if time < 3600.0 {
                date.1 = (time / 60.0).floor();
                time -= date.1 * 60.0;
            } else {
                date.0 = (time / 3600.0).floor();
                time -= date.0 * 3600.0;
            }
        }

        format!("{:02}:{:02}:{:02}", date.0, date.1, date.2)
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
                        if item.transport as u8 == 0 {
                            "TCP"
                        } else {
                            "UDP"
                        }.to_string(),
                        item.bind.to_string(),
                        item.external.to_string(),
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
                Row::new(vec!["TRANSPORT", "BIND", "EXTERNAL"])
                    .style(Style::new().add_modifier(Modifier::DIM)),
            )
            .block(Block::default().title("Interfaces").borders(Borders::ALL)),
            area,
        );
    }
}

struct TablesWidget {
    state: Arc<State>,
    ctx: Arc<Context>,
    popup: PopupWidget,
}

impl TablesWidget {
    fn new(ctx: Arc<Context>, state: Arc<State>) -> Self {
        Self {
            ctx,
            state: state.clone(),
            popup: PopupWidget::new(state),
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let mut addrs = self.ctx.addrs.write().unwrap();
        let previous = self.state.get_previous_users();
        let mut rows = Vec::with_capacity(100);

        addrs.clear();
        for (name, reports) in self.state.get_users().as_ref() {
            for (addr, report) in reports {
                let previous_ = previous
                    .iter()
                    .find(|item| item.0 == name)
                    .and_then(|item| item.1.iter().find(|item| item.0 == addr).map(|item| item.1))
                    .cloned()
                    .unwrap_or_else(|| Report {
                        received_bytes: 0,
                        send_bytes: 0,
                        received_pkts: 0,
                        send_pkts: 0,
                    });

                addrs.push(addr.clone());
                rows.push(Row::new(vec![
                    name.clone(),
                    addr.to_string(),
                    report.received_bytes.to_string(),
                    ((report.received_bytes - previous_.received_bytes) / 5).to_string(),
                    report.send_bytes.to_string(),
                    ((report.send_bytes - previous_.send_bytes) / 5).to_string(),
                    report.received_pkts.to_string(),
                    ((report.received_pkts - previous_.received_pkts) / 5).to_string(),
                    report.send_pkts.to_string(),
                    ((report.send_pkts - previous_.send_pkts) / 5).to_string(),
                ]))
            }
        }

        let mut table_state = TableState::default();
        table_state.select(Some(self.ctx.index.get()));

        self.popup.draw(frame, area);
        frame.render_stateful_widget(
            Table::new(
                rows,
                vec![
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                    Constraint::Percentage(10),
                ],
            )
            .header(
                Row::new(vec![
                    "USERNAME",
                    "SOCKET ADDR",
                    "*RECV BYTES",
                    "*RECV BYTES/S",
                    "*SEND BYTES",
                    "*SEND BYTES/S",
                    "*RECV PKTS",
                    "*RECV PKTS/S",
                    "*SEND PKTS",
                    "*SEND PKTS/S",
                ])
                .style(Style::new().add_modifier(Modifier::DIM)),
            )
            .block(Block::default().title("Users").borders(Borders::ALL))
            .segment_size(SegmentSize::EvenDistribution)
            .highlight_style(Style::new().bg(Color::Green)),
            area,
            &mut table_state,
        );
    }
}

struct PopupWidget {
    state: Arc<State>,
}

impl PopupWidget {
    fn new(state: Arc<State>) -> Self {
        Self { state }
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let session = self.state.get_session();
        if let Some(session) = session.as_ref() {
            frame.render_widget(
                Paragraph::new(
                    [
                        ("Username", session.username.clone()),
                        ("Password", session.password.clone()),
                        ("Lifetime", session.lifetime.to_string()),
                        ("Timer", session.timer.to_string()),
                        (
                            "Channels",
                            session
                                .channels
                                .iter()
                                .map(|item| item.to_string())
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                        (
                            "Ports",
                            session
                                .ports
                                .iter()
                                .map(|item| item.to_string())
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                    ]
                    .into_iter()
                    .map(|(k, v)| Line::from(vec![Span::raw(k).blue(), ": ".into(), Span::raw(v)]))
                    .collect::<Vec<Line>>(),
                )
                .block(
                    Block::default()
                        .title("Info")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .padding(Padding::uniform(1)),
                )
                .alignment(Alignment::Left),
                Self::centered_rect(area, 30, 10),
            );
        }
    }

    fn centered_rect(r: Rect, x: u16, y: u16) -> Rect {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - x) / 2),
                Constraint::Percentage(x),
                Constraint::Percentage((100 - x) / 2),
            ])
            .split(
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(10),
                        Constraint::Length(y),
                        Constraint::Percentage(50),
                    ])
                    .split(r)[1],
            )[1]
    }
}
