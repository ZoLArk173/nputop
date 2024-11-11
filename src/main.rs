use crossterm::{
    event::{poll, read, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition},
    Terminal,
};
use std::fs;
use std::io::{stdout, Result};
use std::time::{Duration, SystemTime};

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut previous_npu_runtime: f64 = 0.0;
    let mut previous_real_time = SystemTime::now();
    let mut usage_history: Vec<(f64, f64)> = Vec::new();
    let mut elapsed_time: f64 = 0.0;

    loop {
        // Read NPU runtime from the specified file
        let npu_runtime =
            fs::read_to_string("/sys/devices/pci0000:00/0000:00:0b.0/power/runtime_active_time")
                .unwrap_or_else(|_| String::from("0"));
        let npu_runtime: f64 = npu_runtime.trim().parse().unwrap_or(0.0);

        // Get the difference between the current runtime and the previous runtime
        let npu_runtime_diff = npu_runtime - previous_npu_runtime;
        previous_npu_runtime = npu_runtime;

        // Get the elapsed real time since the last update (in milliseconds)
        let real_time_diff = previous_real_time
            .elapsed()
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);
        previous_real_time = SystemTime::now();

        // Calculate NPU usage as a percentage (runtime difference / real-time difference)
        let npu_usage_percentage = if real_time_diff > 0.0 {
            (npu_runtime_diff / real_time_diff) * 100.0
        } else {
            0.0
        };

        // Update usage history (keep the last 60 seconds of data)
        elapsed_time += 1.0;
        usage_history.push((elapsed_time, npu_usage_percentage));
        if usage_history.len() > 60 {
            usage_history.remove(0);
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size());

            // Draw the line chart for NPU usage history
            let datasets = vec![Dataset::default()
                .name("NPU Usage %")
                .marker(symbols::Marker::Braille)
                .style(Color::Cyan)
                .graph_type(GraphType::Line)
                .data(&usage_history)];
            let chart = Chart::new(datasets)
                .block(
                    Block::default()
                        .title("NPU Usage History")
                        .borders(Borders::ALL),
                )
                .x_axis(
                    Axis::default()
                        // .title("Time (s)")
                        .style(Color::White)
                        .bounds([elapsed_time - 60.0, elapsed_time])
                        .labels(vec![
                            format!("60"),
                            format!("30"),
                            format!("now"),
                        ]),
                )
                .y_axis(
                    Axis::default()
                        .title("Usage %")
                        .style(Color::White)
                        .bounds([0.0, 100.0])
                        .labels(vec!["0", "50", "100"]),
                )
                .legend_position(None);
            f.render_widget(chart, chunks[0]);
        })?;

        // Handle input here (e.g., break loop on 'q' key press)
        if poll(Duration::from_secs(1))? {
            if let Event::Key(key) = read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    Ok(())
}
