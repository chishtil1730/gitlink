use super::history::{Action, History};
use super::storage::{load_tasks, save_tasks};
use super::task::Task;
use chrono::Local;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType,
        EnterAlternateScreen, LeaveAlternateScreen,
        size,
    },
};
use std::io::{self, Write};

pub fn run_planner() -> Result<(), Box<dyn std::error::Error>> {
    let mut task_list = load_tasks();
    let mut history = History::new();
    let mut selected_index = 0;

    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    enable_raw_mode()?;

    // Initial render
    redraw(&mut stdout, &task_list.tasks, selected_index)?;

    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let mut state_changed = false;

            match key.code {
                KeyCode::Char('q') => break,

                KeyCode::Up => {
                    if selected_index > 0 {
                        selected_index -= 1;
                        state_changed = true;
                    }
                }

                KeyCode::Down => {
                    if selected_index < task_list.tasks.len() {
                        selected_index += 1;
                        state_changed = true;
                    }
                }

                KeyCode::Char(' ') => {
                    if selected_index < task_list.tasks.len() {
                        let task = &mut task_list.tasks[selected_index];
                        history.push(Action::Toggle {
                            id: task.id.clone(),
                        });
                        task.toggle();
                        save_tasks(&task_list)?;
                        state_changed = true;
                    }
                }

                KeyCode::Char('d') => {
                    if selected_index < task_list.tasks.len() {
                        let task = task_list.tasks.remove(selected_index);
                        history.push(Action::Delete {
                            task,
                            index: selected_index,
                        });

                        if selected_index >= task_list.tasks.len()
                            && !task_list.tasks.is_empty()
                        {
                            selected_index = task_list.tasks.len() - 1;
                        }

                        save_tasks(&task_list)?;
                        state_changed = true;
                    }
                }

                KeyCode::Char('a') => {
                    disable_raw_mode()?;
                    execute!(stdout, cursor::Show)?;

                    if let Some(task) = prompt_new_task()? {
                        history.push(Action::Add {
                            task: task.clone(),
                        });
                        task_list.tasks.push(task);
                        save_tasks(&task_list)?;
                        state_changed = true;
                    }

                    enable_raw_mode()?;
                    execute!(stdout, cursor::Hide)?;
                }

                KeyCode::Char('e') => {
                    if selected_index < task_list.tasks.len() {
                        disable_raw_mode()?;
                        execute!(stdout, cursor::Show)?;

                        let task = &mut task_list.tasks[selected_index];

                        if let Some((new_title, new_desc, new_tags)) =
                            prompt_edit_task(&task.title, &task.description, &task.tags)?
                        {
                            task.update_title(new_title);
                            task.update_description(new_desc);
                            task.set_tags(new_tags);
                            save_tasks(&task_list)?;
                            state_changed = true;
                        }

                        enable_raw_mode()?;
                        execute!(stdout, cursor::Hide)?;
                    }
                }

                KeyCode::Char('u') => {
                    if history.undo(&mut task_list.tasks) {
                        save_tasks(&task_list)?;
                        if selected_index >= task_list.tasks.len()
                            && !task_list.tasks.is_empty()
                        {
                            selected_index =
                                task_list.tasks.len().saturating_sub(1);
                        }
                        state_changed = true;
                    }
                }

                KeyCode::Char('r') => {
                    if history.redo(&mut task_list.tasks) {
                        save_tasks(&task_list)?;
                        state_changed = true;
                    }
                }

                _ => {}
            }

            if state_changed {
                redraw(&mut stdout, &task_list.tasks, selected_index)?;
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout, cursor::Show, LeaveAlternateScreen)?;
    Ok(())
}

fn redraw(
    stdout: &mut io::Stdout,
    tasks: &[Task],
    selected: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All)
    )?;

    render_tasks(tasks, selected)?;
    stdout.flush()?;
    Ok(())
}

fn render_tasks(
    tasks: &[Task],
    selected: usize,
) -> Result<(), Box<dyn std::error::Error>> {

    let mut stdout = io::stdout();
    let (width, height) = size()?;
    let width = width as usize;
    let height = height as usize;

    if width < 40 || height < 10 {
        write!(stdout, "Terminal too small. Please resize the window.")?;
        return Ok(());
    }

    let mut output = String::new();

    // HEADER
    output.push_str(&format!("{}\n", "=".repeat(width)));
    output.push_str("📋 GitLink Task Planner\n");
    output.push_str(&format!("{}\n\n", "=".repeat(width)));

    let footer_lines = 5;
    let header_lines = 4;
    let available_height = height
        .saturating_sub(header_lines + footer_lines)
        .max(1);

    let total_items = tasks.len() + 1;

    let half = available_height / 2;
    let start = selected.saturating_sub(half);
    let end = (start + available_height).min(total_items);

    for index in start..end {
        if index < tasks.len() {
            let task = &tasks[index];

            let checkbox = if task.completed { "☑" } else { "☐" };
            let pointer = if index == selected { "→ " } else { "  " };

            let mut line = format!("{}{} {}", pointer, checkbox, task.title);

            if !task.tags.is_empty() {
                line.push_str(&format!(" [{}]", task.tags.join(", ")));
            }

            if line.len() > width {
                line.truncate(width.saturating_sub(1));
            }

            output.push_str(&format!("{}\n", line));

            let created = task.created_at.with_timezone(&Local);
            output.push_str(&format!(
                "     📅 {}\n\n",
                created.format("%Y-%m-%d %H:%M")
            ));
        } else {
            let pointer = if index == selected { "→ " } else { "  " };
            output.push_str(&format!("{}+ Add New Task\n", pointer));
        }
    }

    output.push_str(&format!("\n{}\n", "─".repeat(width)));
    output.push_str("💡 Controls:\n");
    output.push_str("   ↑/↓: Navigate | Space: Toggle | a: Add | e: Edit | d: Delete\n");
    output.push_str("   u: Undo | r: Redo | q: Quit\n");

    write!(stdout, "{}", output)?;
    Ok(())
}



fn prompt_new_task() -> Result<Option<Task>, Box<dyn std::error::Error>> {
    use dialoguer::{theme::ColorfulTheme, Input};

    println!("\r\n📝 Create New Task");
    println!("{}", "=".repeat(80));

    let title: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Task title")
        .interact_text()?;

    if title.trim().is_empty() {
        return Ok(None);
    }

    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact_text()?;

    let tags_input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Tags (comma-separated, optional)")
        .allow_empty(true)
        .interact_text()?;

    let mut task = Task::new(title);

    if !description.trim().is_empty() {
        task.update_description(Some(description));
    }

    if !tags_input.trim().is_empty() {
        let tags: Vec<String> = tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        task.set_tags(tags);
    }

    Ok(Some(task))
}

fn prompt_edit_task(
    current_title: &str,
    current_desc: &Option<String>,
    current_tags: &[String],
) -> Result<Option<(String, Option<String>, Vec<String>)>, Box<dyn std::error::Error>> {
    use dialoguer::{theme::ColorfulTheme, Input};

    println!("\r\n✏️  Edit Task");
    println!("{}", "=".repeat(80));

    let title: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Task title")
        .with_initial_text(current_title)
        .interact_text()?;

    if title.trim().is_empty() {
        return Ok(None);
    }

    let desc_initial = current_desc.as_deref().unwrap_or("");
    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Description (optional)")
        .with_initial_text(desc_initial)
        .allow_empty(true)
        .interact_text()?;

    let tags_initial = current_tags.join(", ");
    let tags_input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Tags (comma-separated)")
        .with_initial_text(&tags_initial)
        .allow_empty(true)
        .interact_text()?;

    let desc_final = if description.trim().is_empty() {
        None
    } else {
        Some(description)
    };

    let tags: Vec<String> = if tags_input.trim().is_empty() {
        Vec::new()
    } else {
        tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    Ok(Some((title, desc_final, tags)))
}