use chrono::{DateTime, Local, TimeZone};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::Rng;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph,
        canvas::{Canvas, Line as CanvasLine, Rectangle},
    },
};
use std::{
    collections::HashMap,
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
struct Candle {
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

enum Message {
    NewCandle(String, Candle),
    Quit,
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();

    let markets: Vec<String> = vec![
        "USD/BTC".to_string(),
        "USD/ETH".to_string(),
        "IDR/BTC".to_string(),
        "IDR/ETH".to_string(),
    ];

    let thread_markets = markets.clone();

    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        
        // Initialize with realistic prices based on provided values
        let mut prices: HashMap<String, f64> = HashMap::new();
        prices.insert("USD/BTC".to_string(), 103879.0);
        prices.insert("USD/ETH".to_string(), 2548.64);
        prices.insert("IDR/BTC".to_string(), 1729998000.0);
        prices.insert("IDR/ETH".to_string(), 42679530.0);
        
        let mut time = Local::now().timestamp();

        loop {
            for market in &thread_markets {
                let price = prices.get_mut(market).unwrap();
                let open = *price;
                
                // Scale the volatility based on price magnitude
                let volatility_factor = match market.as_str() {
                    "USD/BTC" => 100.0,
                    "USD/ETH" => 10.0,
                    "IDR/BTC" => 1000000.0,
                    "IDR/ETH" => 100000.0,
                    _ => 1.0,
                };
                
                let movement = rng.random_range(-1.0..1.0) * volatility_factor;
                *price += movement;

                let high = open.max(*price) + rng.random_range(0.0..volatility_factor * 0.2);
                let low = open.min(*price) - rng.random_range(0.0..volatility_factor * 0.2);
                let close = *price;
                
                // Scale volume based on the market
                let volume_factor = match market.as_str() {
                    "USD/BTC" | "IDR/BTC" => 5.0,
                    "USD/ETH" | "IDR/ETH" => 20.0,
                    _ => 1.0,
                };
                let volume = rng.random_range(100.0..1000.0) * volume_factor;

                let candle = Candle {
                    time,
                    open,
                    high,
                    low,
                    close,
                    volume,
                };

                if tx_clone
                    .send(Message::NewCandle(market.clone(), candle))
                    .is_err()
                {
                    return;
                }
            }

            thread::sleep(Duration::from_secs(1));
            time += 60;
        }
    });

    let mut data: HashMap<String, Vec<Candle>> = HashMap::new();
    let mut price_changes: HashMap<String, f64> = HashMap::new();
    let mut latest_price_map: HashMap<String, f64> = HashMap::new();

    for m in markets.iter() {
        data.insert(m.clone(), Vec::new());
        price_changes.insert(m.clone(), 0.0);
    }

    let mut selected_market = 0;
    let mut should_quit = false;
    let mut last_update = Instant::now();

    while !should_quit {
        if let Ok(message) = rx.try_recv() {
            match message {
                Message::NewCandle(market, candle) => {
                    if let Some(candles) = data.get_mut(&market) {
                        if let Some(last_candle) = candles.last() {
                            let change = candle.close - last_candle.close;
                            if let Some(price_change) = price_changes.get_mut(&market) {
                                *price_change = change;
                            }
                        }

                        candles.push(candle.clone());
                        if candles.len() > 30 {
                            candles.remove(0);
                        }
                    }
                    latest_price_map.insert(market.clone(), candle.close);
                }
                Message::Quit => should_quit = true,
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        tx.send(Message::Quit).unwrap();
                        should_quit = true;
                    }
                    KeyCode::Down => {
                        selected_market = (selected_market + 1) % markets.len();
                    }
                    KeyCode::Up => {
                        selected_market = if selected_market == 0 {
                            markets.len() - 1
                        } else {
                            selected_market - 1
                        };
                    }
                    _ => {}
                }
            }
        }

        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Length(20), Constraint::Min(10)].as_ref())
                .split(size);

            let chart_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(chunks[1]);

            let items: Vec<Line> = markets
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let change = price_changes.get(m).unwrap_or(&0.0);
                    let (icon, color) = if *change > 0.0 {
                        ("🔼", Color::Green)
                    } else if *change < 0.0 {
                        ("🔽", Color::Red)
                    } else {
                        (" ", Color::Gray)
                    };

                    // Format change text appropriately based on market
                    let change_text = if *change != 0.0 {
                        match m.as_str() {
                            "USD/BTC" | "USD/ETH" => format!("({:.2})", change),
                            "IDR/BTC" | "IDR/ETH" => format!("({:.0})", change),
                            _ => format!("({:.2})", change),
                        }
                    } else {
                        String::new()
                    };

                    let market_text = format!("{} {} {}", icon, m, change_text);

                    if i == selected_market {
                        Line::from(Span::styled(
                            market_text,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from(Span::styled(market_text, Style::default().fg(color)))
                    }
                })
                .collect();

            let block = Block::default()
                .title(" Markets ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));

            let paragraph = Paragraph::new(items)
                .block(block)
                .alignment(Alignment::Left);

            f.render_widget(paragraph, chunks[0]);

            let selected = &markets[selected_market];
            if let Some(candles) = data.get(selected) {
                render_candlestick_chart(f, chart_chunks[0], candles);
                render_volume_chart(f, chart_chunks[1], candles);

                if let Some(latest_price) = latest_price_map.get(selected) {
                    let currency = if selected.starts_with("USD") {
                        "USD"
                    } else if selected.starts_with("IDR") {
                        "IDR"
                    } else {
                        ""
                    };

                    let price_text = match currency {
                        "USD" => format!("USD{:>15}", format_usd(*latest_price)),
                        "IDR" => format!("Rp{:>16}", format_idr(*latest_price)),
                        _ => format!("{} {:.2}", currency, latest_price),
                    };

                    let info_block = Paragraph::new(Span::styled(
                        price_text,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .alignment(Alignment::Right);

                    let info_area = Rect {
                        x: chart_chunks[1].x,
                        y: chart_chunks[1].y + chart_chunks[1].height.saturating_sub(1),
                        width: chart_chunks[1].width,
                        height: 1,
                    };

                    f.render_widget(info_block, info_area);
                }
            }
        })?;

        let elapsed = last_update.elapsed();
        if elapsed < Duration::from_millis(100) {
            thread::sleep(Duration::from_millis(100) - elapsed);
        }
        last_update = Instant::now();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_candlestick_chart(f: &mut ratatui::Frame, area: Rect, candles: &[Candle]) {
    if candles.is_empty() {
        f.render_widget(
            Block::default()
                .title("Candlestick Chart")
                .borders(Borders::ALL),
            area,
        );
        return;
    }

    let (min_price, max_price) = candles.iter().fold((f64::MAX, f64::MIN), |(min, max), c| {
        (min.min(c.low), max.max(c.high))
    });

    let y_padding = (max_price - min_price) * 0.1;
    let y_min = min_price - y_padding;
    let y_max = max_price + y_padding;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .title("Candlestick Chart")
                .borders(Borders::ALL),
        )
        .x_bounds([0.0, candles.len() as f64])
        .y_bounds([y_min, y_max])
        .paint(|ctx| {
            for (i, candle) in candles.iter().enumerate() {
                let x = i as f64 + 0.5;

                ctx.draw(&CanvasLine {
                    x1: x,
                    y1: candle.low,
                    x2: x,
                    y2: candle.high,
                    color: Color::White,
                });

                let (body_bottom, body_top) = if candle.close >= candle.open {
                    (candle.open, candle.close)
                } else {
                    (candle.close, candle.open)
                };

                let color = if candle.close >= candle.open {
                    Color::Green
                } else {
                    Color::Red
                };

                ctx.draw(&Rectangle {
                    x: x - 0.3,
                    y: body_bottom,
                    width: 0.6,
                    height: body_top - body_bottom,
                    color,
                });
            }
        });

    f.render_widget(canvas, area);
}

fn render_volume_chart(f: &mut ratatui::Frame, area: Rect, candles: &[Candle]) {
    if candles.is_empty() {
        f.render_widget(Block::default().title("Volume").borders(Borders::ALL), area);
        return;
    }

    let max_volume = candles.iter().map(|c| c.volume).fold(0.0, f64::max) * 1.1;

    let volumes: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.volume))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("Volume")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Bar)
            .style(Style::default().fg(Color::Blue))
            .data(&volumes),
    ];

    let x_labels = if candles.len() > 5 {
        vec![
            Span::from(format_time(candles.first().unwrap().time)),
            Span::from(format_time(candles.last().unwrap().time)),
        ]
    } else {
        candles
            .iter()
            .map(|c| Span::from(format_time(c.time)))
            .collect()
    };

    let y_labels = vec![
        Span::from("0"),
        Span::from(format!("{:.0}", max_volume / 2.0)),
        Span::from(format!("{:.0}", max_volume)),
    ];

    let chart = Chart::new(datasets)
        .block(Block::default().title("Volume").borders(Borders::ALL))
        .x_axis(
            Axis::default()
                .title(Line::from("Time"))
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, candles.len() as f64 - 1.0])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title(Line::from("Volume"))
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, max_volume])
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

fn format_time(timestamp: i64) -> String {
    match DateTime::from_timestamp(timestamp, 0) {
        Some(dt) => {
            let local_dt = Local.from_utc_datetime(&dt.naive_utc());
            local_dt.format("%H:%M").to_string()
        }
        None => {
            eprintln!("Warning: Invalid timestamp {}", timestamp);
            "Invalid Time".to_string()
        }
    }
}

fn format_usd(price: f64) -> String {
    if !price.is_finite() {
        return "Invalid".to_string();
    }

    if price == 0.0 {
        return "$0.00".to_string();
    }

    let abs_price = price.abs();
    let sign = if price < 0.0 { "-" } else { "" };

    let formatted = if abs_price >= 1_000_000_000.0 {
        format!("{}{:.2}B", sign, abs_price / 1_000_000_000.0)
    } else if abs_price >= 1_000_000.0 {
        format!("{}{:.2}M", sign, abs_price / 1_000_000.0)
    } else if abs_price >= 1_000.0 {
        format!("{}{:.2}K", sign, abs_price / 1_000.0)
    } else if abs_price >= 0.10 {
        format!("{}{:.2}", sign, abs_price)
    } else {
        format!("{}{:.4}", sign, abs_price) 
    };

    if abs_price < 1_000.0 && abs_price >= 0.10 {
        let parts: Vec<&str> = formatted.split('.').collect();
        let integer_part = parts[0]
            .chars()
            .rev()
            .collect::<String>()
            .as_bytes()
            .chunks(3)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<&str>>()
            .join(",")
            .chars()
            .rev()
            .collect::<String>();

        format!("${}.{}", integer_part, parts[1])
    } else {
        format!("${}", formatted)
    }
}

fn format_idr(price: f64) -> String {
    if price.is_nan() || price.is_infinite() {
        return "Invalid".to_string();
    }
    
    let rounded = price.round() as i64;
    let mut s = rounded.to_string();
    let mut result = String::new();

    while s.len() > 3 {
        let len = s.len();
        result = format!(".{}{}", &s[len - 3..], result);
        s.truncate(len - 3);
    }

    format!("{}{}", s, result)
}