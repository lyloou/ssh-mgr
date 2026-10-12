use crate::{config::model::Server, search};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::Duration,
};

pub fn run(servers: Vec<Server>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let result = run_app(&mut Terminal::new(CrosstermBackend::new(stdout))?, servers);
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, servers: Vec<Server>) -> Result<()> {
    let mut query = String::new();
    let mut selected = 0usize;
    let mut searching = false;
    loop {
        let matches = search::filter(&servers, &query);
        if selected >= matches.len() && !matches.is_empty() {
            selected = matches.len() - 1;
        }
        terminal.draw(|frame| draw(frame, &matches, selected, &query, searching))?;
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match (code, modifiers, searching) {
                (KeyCode::Char('q'), _, false) => break,
                (KeyCode::Char('/'), _, false) => searching = true,
                (KeyCode::Esc, _, true) => {
                    searching = false;
                    query.clear();
                    selected = 0;
                }
                (KeyCode::Backspace, _, true) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Char(c), _, true) => {
                    query.push(c);
                    selected = 0;
                }
                (KeyCode::Down, _, false) | (KeyCode::Char('j'), _, false) => {
                    if selected + 1 < matches.len() {
                        selected += 1;
                    }
                }
                (KeyCode::Up, _, false) | (KeyCode::Char('k'), _, false) => {
                    selected = selected.saturating_sub(1);
                }
                (KeyCode::Enter, _, false) if !matches.is_empty() => {
                    connect(matches[selected])?;
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL, _) => break,
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, servers: &[&Server], selected: usize, query: &str, searching: bool) {
    let areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(frame.area());
    let items = servers
        .iter()
        .map(|server| {
            ListItem::new(format!(
                "{}/{}  {}@{}:{}",
                server.group, server.name, server.user, server.host, server.port
            ))
        })
        .collect::<Vec<_>>();
    let title = if searching {
        format!("Search: {}", query)
    } else {
        format!(
            "Servers ({})  / search  Enter connect  q quit",
            servers.len()
        )
    };
    let mut state = ListState::default();
    state.select((!servers.is_empty()).then_some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue)),
        areas[0],
        &mut state,
    );
    if let Some(server) = servers.get(selected) {
        let auth = server
            .password
            .as_ref()
            .map(|p| {
                if p.key.is_some() {
                    "key"
                } else if p.env.is_some() {
                    "environment"
                } else if p.value.is_some() {
                    "TOML value"
                } else {
                    "prompt"
                }
            })
            .unwrap_or("default");
        let text = format!(
            "Name: {}\nGroup: {}\nHost: {}:{}\nUser: {}\nAuth: {}\nDescription: {}\nSource: {}",
            server.name,
            server.group,
            server.host,
            server.port,
            server.user,
            auth,
            server.description,
            server.source.display()
        );
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(Block::default().title("Details").borders(Borders::ALL)),
            areas[1],
        );
    }
}

fn connect(server: &Server) -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    let result = connect_ssh(server);
    if let Err(error) = &result {
        eprintln!("SSH failed: {error}");
    }
    eprintln!("Press Enter to return.");
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    result
}

fn connect_ssh(server: &Server) -> Result<()> {
    let tcp = TcpStream::connect((server.host.as_str(), server.port))?;
    let mut session = ssh2::Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    let auth = server.password.as_ref().cloned().unwrap_or_default();
    if let Some(key) = auth.key {
        session.userauth_pubkey_file(
            &server.user,
            None,
            std::path::Path::new(&shellexpand(&key)),
            None,
        )?;
    } else {
        let password = if auth.prompt {
            rpassword::prompt_password("SSH password: ")?
        } else if let Some(value) = auth.value {
            value
        } else if let Some(env) = auth.env {
            std::env::var(&env)
                .map_err(|_| anyhow::anyhow!("environment variable {env} is not set"))?
        } else {
            rpassword::prompt_password("SSH password: ")?
        };
        session.userauth_password(&server.user, &password)?;
    }
    if !session.authenticated() {
        anyhow::bail!("SSH authentication failed");
    }
    session.set_blocking(false);
    let mut channel = session.channel_session()?;
    channel.request_pty("xterm", None, Some((120, 40, 0, 0)))?;
    channel.shell()?;
    let mut stdout = io::stdout();
    let mut buffer = [0u8; 4096];
    loop {
        match channel.read(&mut buffer) {
            Ok(size) if size > 0 => {
                stdout.write_all(&buffer[..size])?;
                stdout.flush()?;
            }
            Ok(_) if channel.eof() => break,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
            _ => {}
        }
        if event::poll(Duration::from_millis(20))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => channel.write_all(c.to_string().as_bytes())?,
                    KeyCode::Enter => channel.write_all(b"\r")?,
                    KeyCode::Backspace => channel.write_all(&[8])?,
                    KeyCode::Tab => channel.write_all(b"\t")?,
                    KeyCode::Esc => channel.write_all(&[27])?,
                    KeyCode::Up => channel.write_all(b"\x1b[A")?,
                    KeyCode::Down => channel.write_all(b"\x1b[B")?,
                    KeyCode::Right => channel.write_all(b"\x1b[C")?,
                    KeyCode::Left => channel.write_all(b"\x1b[D")?,
                    _ => {}
                }
            }
        }
    }
    channel.wait_close()?;
    Ok(())
}

fn shellexpand(path: &str) -> String {
    path.strip_prefix("~/")
        .and_then(|rest| dirs::home_dir().map(|home| home.join(rest).display().to_string()))
        .unwrap_or_else(|| path.to_string())
}
