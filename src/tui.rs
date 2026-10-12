use crate::i18n::text as tr;
use crate::{
    config::{model::Server, scanner::scan},
    search,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    collections::{BTreeMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn run(servers: Vec<Server>, config_dir: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let result = run_app(
        &mut Terminal::new(CrosstermBackend::new(stdout))?,
        servers,
        config_dir,
    );
    disable_raw_mode()?;
    execute!(io::stdout(), crossterm::cursor::Show, LeaveAlternateScreen)?;
    result
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut servers: Vec<Server>,
    config_dir: PathBuf,
) -> Result<()> {
    let mut query = String::new();
    let mut selected = 0usize;
    let mut searching = false;
    let mut help = false;
    let mut collapsed = HashSet::new();
    let mut status = String::new();
    let mut list_state = ListState::default();
    let mut page_height = 1;
    let mut pending_g = false;
    loop {
        let rows = visible_rows(&servers, &query, &collapsed);
        selected = selected.min(rows.len().saturating_sub(1));
        terminal.draw(|frame| {
            draw(
                frame,
                &rows,
                selected,
                &query,
                searching,
                &status,
                help,
                &mut list_state,
                &mut page_height,
            )
        })?;
        // A detached tmux session can resize during attach without a queued key event.
        // Draw periodically so Ratatui rechecks the actual terminal size.
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            if !help {
                if let Some(next) =
                    page_selection(code, modifiers, selected, rows.len(), page_height)
                {
                    selected = next;
                    pending_g = false;
                    continue;
                }
            }
            if !searching && !help && code == KeyCode::Char('g') && modifiers.is_empty() {
                if pending_g {
                    selected = 0;
                }
                pending_g = !pending_g;
                continue;
            }
            pending_g = false;
            match (code, modifiers, searching, help) {
                (KeyCode::Char('G'), _, false, false) => selected = rows.len().saturating_sub(1),
                (KeyCode::Char('q'), _, false, false) => break,
                (KeyCode::Char('/'), _, false, false) => searching = true,
                (KeyCode::Char('?'), _, false, false) => help = true,
                (KeyCode::Esc, _, _, true) | (KeyCode::Char('?'), _, _, true) => help = false,
                (KeyCode::Esc, _, true, false) => {
                    searching = false;
                    query.clear();
                    selected = 0;
                }
                (KeyCode::Enter, _, true, false) => {
                    searching = false;
                }
                (KeyCode::Backspace, _, true, false) => {
                    query.pop();
                    selected = 0;
                }
                (KeyCode::Down, _, true, false) => {
                    if selected + 1 < rows.len() {
                        selected += 1;
                    }
                }
                (KeyCode::Up, _, true, false) => {
                    selected = selected.saturating_sub(1);
                }
                (KeyCode::Char(c), _, true, false) => {
                    query.push(c);
                    selected = 0;
                }
                (KeyCode::Down, _, false, false) | (KeyCode::Char('j'), _, false, false) => {
                    if selected + 1 < rows.len() {
                        selected += 1;
                    }
                }
                (KeyCode::Up, _, false, false) | (KeyCode::Char('k'), _, false, false) => {
                    selected = selected.saturating_sub(1);
                }
                (KeyCode::Left, _, false, false) | (KeyCode::Char('h'), _, false, false) => {
                    if let Some(group_index) = toggle_group(&rows, selected, &mut collapsed, true) {
                        selected = group_index;
                    }
                }
                (KeyCode::Right, _, false, false) | (KeyCode::Char('l'), _, false, false) => {
                    toggle_group(&rows, selected, &mut collapsed, false);
                }
                (KeyCode::Char('r'), _, false, false) => match scan(&config_dir) {
                    Ok(updated) => {
                        servers = updated;
                        selected = 0;
                        status = format!("Reloaded {} servers", servers.len());
                    }
                    Err(error) => status = format!("Reload failed: {error}"),
                },
                (KeyCode::Char(action @ ('e' | 'a')), _, false, false) => {
                    let path = if action == 'e' {
                        match rows.get(selected) {
                            Some(Row::Server(server)) => Ok(server.source.clone()),
                            _ => Err(anyhow::anyhow!("Select a server to edit")),
                        }
                    } else {
                        let group = match rows.get(selected) {
                            Some(Row::Group { name, .. }) => Some(name.as_str()),
                            Some(Row::Server(server)) => Some(server.group.as_str()),
                            None => None,
                        };
                        create_server_file(&config_dir, group)
                    };
                    match path.and_then(|path| edit_config(&path)) {
                        Ok(()) => match scan(&config_dir) {
                            Ok(updated) => {
                                servers = updated;
                                selected = 0;
                                status = format!("Reloaded {} servers", servers.len());
                            }
                            Err(error) => status = format!("Reload failed: {error}"),
                        },
                        Err(error) => status = error.to_string(),
                    }
                    terminal.clear()?;
                }
                (KeyCode::Char('o'), _, false, false) => {
                    if let Some(Row::Server(server)) = rows.get(selected) {
                        if let Err(error) = launch_ssh(server, false) {
                            status = format!("SSH failed: {error}");
                        }
                        terminal.clear()?;
                    }
                }
                (KeyCode::Char('O'), _, false, false) => {
                    if let Some(Row::Server(server)) = rows.get(selected) {
                        if let Err(error) = launch_ssh(server, true) {
                            status = format!("SSH failed: {error}");
                        }
                        terminal.clear()?;
                    }
                }
                (KeyCode::Enter, _, false, false) => {
                    if let Some(Row::Server(server)) = rows.get(selected) {
                        if let Err(error) = launch_ssh(server, false) {
                            status = format!("SSH failed: {error}");
                        }
                        terminal.clear()?;
                    }
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL, _, _) => break,
                _ => {}
            }
        }
    }
    Ok(())
}

enum Row<'a> {
    Group {
        name: String,
        count: usize,
        collapsed: bool,
    },
    Server(&'a Server),
}

fn page_selection(
    code: KeyCode,
    modifiers: KeyModifiers,
    selected: usize,
    length: usize,
    height: usize,
) -> Option<usize> {
    let height = height.max(1);
    let (down, step) = match code {
        KeyCode::PageDown => (true, height),
        KeyCode::PageUp => (false, height),
        KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => (true, height),
        KeyCode::Char('b') if modifiers.contains(KeyModifiers::CONTROL) => (false, height),
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            (true, (height / 2).max(1))
        }
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            (false, (height / 2).max(1))
        }
        _ => return None,
    };
    Some(
        if down {
            selected.saturating_add(step)
        } else {
            selected.saturating_sub(step)
        }
        .min(length.saturating_sub(1)),
    )
}

fn visible_rows<'a>(
    servers: &'a [Server],
    query: &str,
    collapsed: &HashSet<String>,
) -> Vec<Row<'a>> {
    let matches = search::filter(servers, query);
    let mut groups = BTreeMap::<String, Vec<&Server>>::new();
    for server in matches {
        groups.entry(server.group.clone()).or_default().push(server);
    }
    let mut rows = Vec::new();
    for (name, group_servers) in groups {
        let is_collapsed = collapsed.contains(&name);
        rows.push(Row::Group {
            name,
            count: group_servers.len(),
            collapsed: is_collapsed,
        });
        if !is_collapsed {
            rows.extend(group_servers.into_iter().map(Row::Server));
        }
    }
    rows
}

fn toggle_group(
    rows: &[Row<'_>],
    selected: usize,
    collapsed: &mut HashSet<String>,
    collapse: bool,
) -> Option<usize> {
    let group = match rows.get(selected) {
        Some(Row::Group { name, .. }) => name.clone(),
        Some(Row::Server(server)) => server.group.clone(),
        None => return None,
    };
    if collapse {
        collapsed.insert(group.clone());
        return rows
            .iter()
            .position(|row| matches!(row, Row::Group { name, .. } if name == &group));
    }
    collapsed.remove(&group);
    None
}

#[allow(clippy::too_many_arguments)]
fn draw(
    frame: &mut Frame,
    rows: &[Row<'_>],
    selected: usize,
    query: &str,
    searching: bool,
    status: &str,
    help: bool,
    state: &mut ListState,
    page_height: &mut usize,
) {
    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let header = if searching {
        format!(" ssh-mgr  •  tmux: ssh-mgr  •  Search: {}", query)
    } else if query.is_empty() {
        format!(
            " ssh-mgr  •  tmux: ssh-mgr  •  {} servers",
            rows.iter()
                .filter(|row| matches!(row, Row::Server(_)))
                .count()
        )
    } else {
        format!(
            " ssh-mgr  •  {} matches  •  Search: {}",
            rows.iter()
                .filter(|row| matches!(row, Row::Server(_)))
                .count(),
            query
        )
    };
    frame.render_widget(
        Paragraph::new(tr(&header))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
        page[0],
    );
    let areas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(page[1]);
    let items = rows
        .iter()
        .map(|row| {
            ListItem::new(match row {
                Row::Group {
                    name,
                    count,
                    collapsed,
                } => Line::from(vec![
                    Span::styled(
                        if *collapsed { "▸ " } else { "▾ " },
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        name,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {}", count), Style::default().fg(Color::DarkGray)),
                ]),
                Row::Server(server) => Line::from(vec![
                    Span::styled("  ↳ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&server.name, Style::default().fg(Color::White)),
                    Span::styled(
                        format!("  {}@{}:{}", server.user, server.host, server.port),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            })
        })
        .collect::<Vec<_>>();
    let title = if searching {
        format!("Search: {} · Enter 确认", query)
    } else {
        format!(
            "Servers ({})",
            rows.iter()
                .filter(|row| matches!(row, Row::Server(_)))
                .count()
        )
    };
    *page_height = usize::from(areas[0].height.saturating_sub(2)).max(1);
    state.select((!rows.is_empty()).then_some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(tr(&title))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(38, 72, 120))
                    .fg(Color::White),
            ),
        areas[0],
        state,
    );
    let right_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(17)])
        .split(areas[1]);
    if let Some(Row::Server(server)) = rows.get(selected) {
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
            Paragraph::new(tr(&text))
                .wrap(Wrap { trim: false })
                .block(Block::default().title(tr("Details")).borders(Borders::ALL)),
            right_areas[0],
        );
    } else if let Some(Row::Group {
        name,
        count,
        collapsed,
    }) = rows.get(selected)
    {
        frame.render_widget(
            Paragraph::new(tr(&format!(
                "Group: {}\nServers shown: {}\nState: {}",
                name,
                count,
                if *collapsed { "collapsed" } else { "expanded" }
            )))
            .block(Block::default().title(tr("Details")).borders(Borders::ALL)),
            right_areas[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new(tr("No server selected"))
                .block(Block::default().title(tr("Details")).borders(Borders::ALL)),
            right_areas[0],
        );
    }
    let help_text = "↑/↓、j/k  移动\n←/→、h/l  折叠或展开分组\nCtrl-f/b  整页翻动\nCtrl-d/u  半页翻动\ngg / G  首行 / 末行\no  进入 SSH\nO  后台打开 SSH\nEnter  确认搜索 / 进入 SSH\n/  搜索\nEsc  清除搜索或返回\nr  重新扫描配置\ne  Vim 编辑配置\na  新增服务器\nq  退出\n?  显示快捷键";
    frame.render_widget(
        Paragraph::new(tr(help_text))
            .block(Block::default().title(tr("操作")).borders(Borders::ALL)),
        right_areas[1],
    );
    let footer = if status.is_empty() {
        " ↑/↓ j/k move  ←/→ h/l group  o enter  O background  / search  ? help  q quit".to_string()
    } else {
        format!(" {}", status)
    };
    frame.render_widget(
        Paragraph::new(tr(&footer)).style(Style::default().fg(if status.is_empty() {
            Color::DarkGray
        } else {
            Color::Yellow
        })),
        page[2],
    );
    if help {
        let area = centered_rect(70, 70, page[1]);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(tr("↑/↓、j/k  移动\n←/→、h/l  折叠或展开分组\nCtrl-f/b  整页翻动\nCtrl-d/u  半页翻动\ngg / G  首行 / 末行\no  进入 SSH\nO  后台打开 SSH\nEnter  确认搜索 / 进入 SSH\n/  搜索\nEsc  清除搜索或返回\nr  重新扫描配置\ne  Vim 编辑配置\na  新增服务器\nq  退出\n?  关闭帮助"))
                .block(Block::default().title(tr("快捷键")).borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn create_server_file(config_dir: &Path, group: Option<&str>) -> Result<PathBuf> {
    use std::io::Write;
    let directory = match group {
        Some(group) => config_dir.join(group),
        None => config_dir.to_path_buf(),
    };
    fs::create_dir_all(&directory)?;
    let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = directory.join(format!("server-{id}.toml"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(b"# Replace the example fields before connecting.\n[[server]]\nname = \"new-server\"\nhost = \"server.example.com\"\nport = 22\nuser = \"deploy\"\ndescription = \"\"\n\n[server.password]\nprompt = true\n")?;
    Ok(path)
}

fn edit_config(path: &Path) -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), crossterm::cursor::Show, LeaveAlternateScreen)?;
    let result = Command::new("vim").arg("--").arg(path).status();
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide)?;
    let status = result.map_err(|error| anyhow::anyhow!("Unable to start vim: {error}"))?;
    if !status.success() {
        anyhow::bail!("Vim exited with {status}");
    }
    Ok(())
}

fn launch_ssh(server: &Server, background: bool) -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), crossterm::cursor::Show, LeaveAlternateScreen)?;
    let result = match build_ssh_wrapper(server) {
        Ok(wrapper) => {
            let result = launch_tmux(&wrapper, server, background);
            if result.is_err() {
                let _ = fs::remove_dir_all(wrapper.parent().unwrap_or(wrapper.as_path()));
            }
            result
        }
        Err(error) => Err(error),
    };
    let restore = enable_raw_mode()
        .and_then(|_| execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide));
    match (result, restore) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn build_ssh_wrapper(server: &Server) -> Result<PathBuf> {
    let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("ssh-mgr-{id}"));
    fs::create_dir(&dir)?;
    let wrapper = dir.join("run.sh");
    let helper = dir.join("askpass.sh");
    let password_file = dir.join("password");
    let mut args = vec!["ssh".to_string(), "-p".to_string(), server.port.to_string()];
    let auth = server.password.as_ref().cloned().unwrap_or_default();
    let _ = auth.prompt;
    let use_askpass = if let Some(key) = auth.key {
        args.extend(["-i".to_string(), shellexpand(&key)]);
        false
    } else if let Some(value) = auth.value {
        fs::write(&password_file, value)?;
        set_private(&password_file)?;
        true
    } else if let Some(env) = auth.env {
        let value = std::env::var(&env)
            .map_err(|_| anyhow::anyhow!("environment variable {env} is not set"))?;
        fs::write(&password_file, value)?;
        set_private(&password_file)?;
        true
    } else {
        false
    };
    args.push(format!("{}@{}", server.user, server.host));
    let mut script = String::from("#!/bin/sh\nset -u\n");
    script.push_str(&format!(
        "trap 'rm -rf {}' EXIT\n",
        shell_quote(&dir.display().to_string())
    ));
    if use_askpass {
        fs::write(
            &helper,
            format!(
                "#!/bin/sh\ncat {}\n",
                shell_quote(&password_file.display().to_string())
            ),
        )?;
        set_executable(&helper)?;
        script.push_str(&format!(
            "export SSH_ASKPASS={}\n",
            shell_quote(&helper.display().to_string())
        ));
        script.push_str("export SSH_ASKPASS_REQUIRE=force\nexport DISPLAY=ssh-mgr\n");
    }
    script.push_str(
        &args
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" "),
    );
    script.push('\n');
    fs::write(&wrapper, script)?;
    set_executable(&wrapper)?;
    Ok(wrapper)
}

fn launch_tmux(wrapper: &Path, server: &Server, background: bool) -> Result<()> {
    let name = format!("{}__{}", server.group.replace('/', "_"), server.name);
    let inside = std::env::var_os("TMUX").is_some();
    if !inside {
        let exists = Command::new("tmux")
            .args(["has-session", "-t", "ssh-mgr"])
            .status()?
            .success();
        if !exists {
            Command::new("tmux")
                .args(["new-session", "-d", "-s", "ssh-mgr", "-n", "manager"])
                .status()?;
        }
    }
    let mut command = Command::new("tmux");
    command.arg("new-window");
    if background {
        command.arg("-d");
    }
    if !inside {
        command.args(["-t", "ssh-mgr:"]);
    }
    let status = command
        .args(["-n", &name, wrapper.to_str().unwrap()])
        .status()?;
    if !status.success() {
        anyhow::bail!("tmux failed to create SSH window");
    }
    if !inside && !background {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", "ssh-mgr"])
            .status()?;
        if !status.success() {
            anyhow::bail!("tmux failed to attach ssh-mgr session");
        }
    }
    Ok(())
}

fn set_executable(path: &PathBuf) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn set_private(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shellexpand(path: &str) -> String {
    path.strip_prefix("~/")
        .and_then(|rest| dirs::home_dir().map(|home| home.join(rest).display().to_string()))
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn page_navigation_clamps_to_filtered_list_boundaries() {
        use super::{page_selection, KeyCode, KeyModifiers};
        assert_eq!(
            page_selection(KeyCode::Char('f'), KeyModifiers::CONTROL, 0, 50, 10),
            Some(10)
        );
        assert_eq!(
            page_selection(KeyCode::Char('d'), KeyModifiers::CONTROL, 10, 50, 10),
            Some(15)
        );
        assert_eq!(
            page_selection(KeyCode::Char('u'), KeyModifiers::CONTROL, 15, 50, 10),
            Some(10)
        );
        assert_eq!(
            page_selection(KeyCode::Char('b'), KeyModifiers::CONTROL, 3, 50, 10),
            Some(0)
        );
        assert_eq!(
            page_selection(KeyCode::PageDown, KeyModifiers::NONE, 0, 3, 10),
            Some(2)
        );
        assert_eq!(
            page_selection(KeyCode::PageUp, KeyModifiers::NONE, 0, 0, 0),
            Some(0)
        );
        assert_eq!(
            page_selection(KeyCode::Char('f'), KeyModifiers::NONE, 0, 50, 10),
            None
        );
    }

    #[test]
    fn rendering_scrolls_to_selection_and_preserves_offset() {
        let servers: Vec<_> = (0..50)
            .map(|i| server("prod", &format!("server-{i}")))
            .collect();
        let rows = visible_rows(&servers, "", &HashSet::new());
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        let mut state = ratatui::widgets::ListState::default();
        let mut height = 0;
        terminal
            .draw(|frame| {
                super::draw(
                    frame,
                    &rows,
                    40,
                    "",
                    false,
                    "",
                    false,
                    &mut state,
                    &mut height,
                )
            })
            .unwrap();
        assert!(state.offset() > 0);
        assert!(40 >= state.offset() && 40 < state.offset() + height);
        let offset = state.offset();
        terminal
            .draw(|frame| {
                super::draw(
                    frame,
                    &rows,
                    40,
                    "",
                    false,
                    "",
                    false,
                    &mut state,
                    &mut height,
                )
            })
            .unwrap();
        assert_eq!(state.offset(), offset);
    }
    #[test]
    fn adding_servers_creates_distinct_valid_files_in_group() {
        let root = std::env::temp_dir().join(format!(
            "ssh-mgr-add-{}",
            super::SystemTime::now()
                .duration_since(super::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = super::create_server_file(&root, Some("prod")).unwrap();
        let second = super::create_server_file(&root, Some("prod")).unwrap();
        assert_ne!(first, second);
        let servers = super::scan(&root).unwrap();
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().all(|server| server.group == "prod"));
        std::fs::remove_dir_all(root).unwrap();
    }
    use super::{toggle_group, visible_rows, Row};
    use crate::config::model::Server;
    use std::{collections::HashSet, path::PathBuf};

    fn server(group: &str, name: &str) -> Server {
        Server {
            name: name.into(),
            host: "localhost".into(),
            port: 22,
            user: "root".into(),
            description: String::new(),
            tags: Vec::new(),
            password: None,
            group: group.into(),
            source: PathBuf::new(),
        }
    }

    #[test]
    fn collapsed_group_keeps_a_representative_row() {
        let servers = vec![
            server("prod", "api"),
            server("prod", "db"),
            server("staging", "api"),
        ];
        let collapsed = HashSet::from([String::from("prod")]);
        let rows = visible_rows(&servers, "", &collapsed);
        assert!(matches!(
            rows[0],
            Row::Group {
                collapsed: true,
                ..
            }
        ));
        assert!(matches!(rows[1], Row::Group { ref name, .. } if name == "staging"));
        assert!(matches!(rows[2], Row::Server(server) if server.name == "api"));
    }

    #[test]
    fn password_value_is_not_written_to_ssh_command_script() {
        let mut server = server("prod", "api");
        server.password = Some(crate::config::model::PasswordConfig {
            value: Some("secret-value".into()),
            ..Default::default()
        });
        let wrapper = super::build_ssh_wrapper(&server).unwrap();
        let script = std::fs::read_to_string(&wrapper).unwrap();
        assert!(!script.contains("secret-value"));
        std::fs::remove_dir_all(wrapper.parent().unwrap()).unwrap();
    }

    #[test]
    fn collapsing_a_child_returns_its_group_row() {
        let servers = vec![server("prod", "api"), server("prod", "db")];
        let mut collapsed = HashSet::new();
        let rows = visible_rows(&servers, "", &collapsed);
        let group_index = toggle_group(&rows, 2, &mut collapsed, true);
        assert_eq!(group_index, Some(0));
        assert!(collapsed.contains("prod"));
    }
}
