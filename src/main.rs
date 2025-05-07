use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::{
    Frame, Terminal,
    crossterm::event::{self, Event, KeyCode},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition, Paragraph, Wrap},
};

const USD_TO_IDR: f64 = 16500.0;

#[derive(Clone, Copy)]
enum Page {
    Usd,
    Idr,
}

struct App {
    bitcoin_prices: Vec<(f64, f64)>,
    last_price: f64,
    price_change: f64,
    time_counter: f64,
    base_price: f64,
    current_page: Page,
}

impl App {
    fn new() -> Self {
        let base_price = 94448.0;
        let mut prices = Vec::new();

        for i in 0..20 {
            let fluctuation = (rand::random::<f64>() - 0.5) * 40.0;
            prices.push((i as f64, base_price + fluctuation));
        }

        Self {
            bitcoin_prices: prices,
            last_price: base_price,
            price_change: 0.0,
            time_counter: 20.0,
            base_price,
            current_page: Page::Usd,
        }
    }

    fn run(
        mut self,
        mut terminal: Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        let tick_rate = Duration::from_secs(1);
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Tab => {
                            self.current_page = match self.current_page {
                                Page::Usd => Page::Idr,
                                Page::Idr => Page::Usd,
                            };
                        }
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    fn on_tick(&mut self) {
        self.time_counter += 1.0;

        if rand::random::<f64>() < 0.1 {
            self.base_price += (rand::random::<f64>() - 0.45) * 50.0;
        }

        let fluctuation = (rand::random::<f64>() - 0.5) * 40.0;
        let new_price = (self.base_price + fluctuation).max(1.0);

        self.price_change = new_price - self.last_price;
        self.last_price = new_price;
        self.bitcoin_prices.push((self.time_counter, new_price));

        if self.bitcoin_prices.len() > 100 {
            self.bitcoin_prices.drain(0..1);
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(85), Constraint::Percentage(15)])
            .split(frame.area());

        self.render_chart(frame, chunks[0]);
        self.render_info_box(frame, chunks[1]);
    }

    fn render_info_box(&self, frame: &mut Frame, area: Rect) {
        let usd_price = self.last_price;
        let idr_price = self.last_price * USD_TO_IDR;

        // Shorter info box with more compact presentation
        let info = vec![
            Line::styled("USD: $", Style::default().fg(Color::Gray)),
            Line::styled(
                format!("{:.2}", usd_price),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::styled("IDR: Rp", Style::default().fg(Color::Gray)),
            Line::styled(
                format!("{:.2}", idr_price),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::styled("[Tab]Switch", Style::default().fg(Color::Blue)),
            Line::styled("[q]Quit", Style::default().fg(Color::Red)),
        ];

        let paragraph = Paragraph::new(info)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_chart(&self, frame: &mut Frame, area: Rect) {
        let (prices, y_title, chart_title, display_price, display_change) = match self.current_page
        {
            Page::Usd => (
                self.bitcoin_prices.clone(),
                "Price (USD)",
                "Bitcoin Price in USD",
                self.last_price,
                self.price_change,
            ),
            Page::Idr => (
                self.bitcoin_prices
                    .iter()
                    .map(|(x, y)| (*x, y * USD_TO_IDR))
                    .collect(),
                "Price (IDR)",
                "Bitcoin Price in IDR",
                self.last_price * USD_TO_IDR,
                self.price_change * USD_TO_IDR,
            ),
        };

        let min_price = prices
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::INFINITY, f64::min)
            .max(0.0)
            * 0.998;

        let max_price = prices.iter().map(|(_, y)| *y).fold(0.0, f64::max) * 1.002;

        let window_start = (self.time_counter - 20.0).max(0.0);
        let window_end = self.time_counter;

        let x_labels = vec![
            Span::styled(
                format!("{:.1}", window_start),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.1}", (window_start + window_end) / 2.0)),
            Span::styled(
                format!("{:.1}", window_end),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];

        let y_labels = vec![
            Span::styled(
                format!("{:.2}", min_price),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.2}", (min_price + max_price) / 2.0)),
            Span::styled(
                format!("{:.2}", max_price),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];

        let dataset = Dataset::default()
            .name(chart_title)
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(if display_change >= 0.0 {
                Color::Green
            } else {
                Color::Red
            }))
            .data(&prices);

        let change_symbol = if display_change > 0.0 {
            "▲"
        } else if display_change < 0.0 {
            "▼"
        } else {
            "■"
        };
        let change_color = if display_change > 0.0 {
            Color::Green
        } else if display_change < 0.0 {
            Color::Red
        } else {
            Color::White
        };

        let title = Line::from(vec![
            Span::styled("Bitcoin Price: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.2}", display_price),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{} {:.2}", change_symbol, display_change.abs()),
                Style::default().fg(change_color),
            ),
            Span::raw(" ("),
            Span::styled(
                format!("{:.2}%", (display_change / display_price) * 100.0),
                Style::default().fg(change_color),
            ),
            Span::raw(") | "),
            Span::styled(
                "Tab = switch page | q = quit",
                Style::default().fg(Color::Gray),
            ),
        ]);

        let chart = Chart::new(vec![dataset])
            .block(
                Block::default()
                    .title(title)
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("Time (s)")
                    .style(Style::default().fg(Color::Gray))
                    .labels(x_labels)
                    .bounds([window_start, window_end]),
            )
            .y_axis(
                Axis::default()
                    .title(y_title)
                    .style(Style::default().fg(Color::Gray))
                    .labels(y_labels)
                    .bounds([min_price, max_price]),
            )
            .legend_position(Some(LegendPosition::Bottom));

        frame.render_widget(chart, area);
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    let res = App::new().run(terminal);

    crossterm::terminal::disable_raw_mode()?;
    res
}
