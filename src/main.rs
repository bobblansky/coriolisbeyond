use serde_json::{Value};
use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap
    },
    Terminal,
};

const SKILL_DB_PATH: &str = "./data/skills.json";
const CHARACTER_DB_PATH: &str = "./data/character.json";
const WEAPON_DB_PATH: &str = "./data/weapons.json";
const ITEM_DB_PATH: &str = "./data/items.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Serialize, Deserialize, Clone)]
struct Character {
    id: usize,
    name: String,
    experience: u8,
    class: String,
    icon: String,
    background: String,
    skill_ids: Vec<usize>
}

#[derive(Serialize, Deserialize, Clone)]
struct Skill {
    id: usize,
    name: String,
    description: String,
    cost: usize
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Character,
    Skills,
}
impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Character => 1,
            MenuItem::Skills => 2,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });


    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Coriolis", "Character", "Skills", "Quit"];
    let mut active_menu_item = MenuItem::Home;
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);
            let copyright = Paragraph::new("Coriolis Beyond 2022 - No rights reserved")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        //.title("")
                        .border_type(BorderType::Plain),
                );

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            match active_menu_item {
                //MenuItem::Character => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Home => {
                    rect.render_widget(render_home(), chunks[1]);
                },
                MenuItem::Character => {
                    //Big chunk, displays enitre character screen
                    let character_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), 
                            Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    //Divides the middle block into vertical blocks for items/skills/etc
                    let inside_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        //.vertical_margin(1)
                        .constraints([
                            Constraint::Percentage(25), 
                            Constraint::Percentage(25), 
                            Constraint::Percentage(50)].as_ref(),
                        )
                        .split(character_chunks[1]);
                    //get the character and render
                    //left => name
                    //right list character info from json
                    let (left, right, char_skills_ids) = render_character(&list_state);
                    let char_skills = render_character_skills(char_skills_ids);

                    rect.render_widget(Paragraph::new("Utrustning").block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(Style::default().fg(Color::White))
                            .title("Utrustning")
                            .border_type(BorderType::Plain),
                    ), inside_chunks[2]);
                    rect.render_stateful_widget(left, character_chunks[0], &mut list_state);
                    rect.render_widget(char_skills, inside_chunks[1]);
                    rect.render_widget(right, inside_chunks[0]);

                },
                MenuItem::Skills => {
                    let skill_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    //get the skills and render
                    let (left, right) = render_skills(&list_state);
                    //println!("{:?}", left.len());
                    rect.render_stateful_widget(left, skill_chunks[0], &mut list_state);
                    rect.render_widget(right, skill_chunks[1]);
                }
            }
            rect.render_widget(copyright, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('h') => active_menu_item = MenuItem::Home,
                KeyCode::Char('c') => active_menu_item = MenuItem::Character,
                KeyCode::Char('s') => active_menu_item = MenuItem::Skills,
                KeyCode::Char('a') => {
                    //add_skill/item/whatever_to_db().expect("can add new random item");
                    println!("add_random_item_to_db");
                }
                KeyCode::Char('d') => {
                    //remove_item_at_index(&mut item_list_state).expect("can remove item");
                    println!("remove_item_at_index");
                }
                KeyCode::Down => {
                    if active_menu_item == MenuItem::Skills {
                        if let Some(selected) = list_state.selected() {
                            let amount_skills = read_skill_db().expect("can fetch list").len();
                            if selected >= amount_skills - 1 {
                                list_state.select(Some(0));
                            } else {
                                list_state.select(Some(selected + 1));
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Character {
                        if let Some(selected) = list_state.selected() {
                            let amount_skills = read_character_db().expect("can fetch list").len();
                            if selected >= amount_skills - 1 {
                                list_state.select(Some(0));
                            } else {
                                list_state.select(Some(selected + 1));
                            }
                        }
                    }
                }
                KeyCode::Up => {
                    if active_menu_item == MenuItem::Skills {
                        if let Some(selected) = list_state.selected() {
                            let amount_skills = read_character_db().expect("can fetch list").len();
                            if selected > 0 {
                                list_state.select(Some(selected - 1));
                            } else {
                                list_state.select(Some(amount_skills - 1));
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Character {
                        if let Some(selected) = list_state.selected() {
                            let amount_skills = read_character_db().expect("can fetch list").len();
                            if selected > 0 {
                                list_state.select(Some(selected - 1));
                            } else {
                                list_state.select(Some(amount_skills - 1));
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "Coriolis Beyond",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 's' to view skills, 'c' to access characters, 'h' to go home and 'q' to quit.")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );
    home
}

fn render_character<'a>(list_state: &ListState) -> (List<'a>, Table<'a>, Vec<usize>) {
    //TODO:
    // Render items/skills for selected character
    let character = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Character")
        .border_type(BorderType::Plain);

    let character_list = read_character_db().expect("can fetch skill list");

    let items: Vec<_> = character_list
        .iter()
        .map(|character| {
            ListItem::new(Spans::from(vec![Span::styled(
                character.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_character = character_list
        .get(
            list_state
                .selected()
                .expect("there is always a selected skill"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(character).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let character_detail = Table::new(vec![
        Row::new(vec![Cell::from(Span::raw(selected_character.class))]),
        Row::new(vec![Cell::from(Span::raw(selected_character.background))]),
    ])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            selected_character.name,
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Karaktär")
            .border_type(BorderType::Plain),
    )
    .widths(&[Constraint::Percentage(100)]);

    (list, character_detail, selected_character.skill_ids)
}

fn render_skills<'a>(list_state: &ListState) -> (List<'a>, Paragraph<'a>) {
    let skills = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Talanger")
        .border_type(BorderType::Plain);

    let skill_list = read_skill_db().expect("can fetch skill list");
    let items: Vec<_> = skill_list
        .iter()
        .map(|skill| {
            ListItem::new(Spans::from(vec![Span::styled(
                skill.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_skill = skill_list
        .get(
            list_state
                .selected()
                .expect("there is always a selected skill"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(skills).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let skill_detail = Paragraph::new(selected_skill.description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title(selected_skill.name)
                .border_type(BorderType::Plain),
        );

    (list, skill_detail)
}

fn render_character_skills<'a>(char_skills: Vec<usize>) -> (Table<'a>) {
    let skill_list = read_skill_db().expect("can fetch skill list");

    let mut rows: Vec<Row> = Vec::new();

    for skill in skill_list {
        if char_skills.contains(&skill.id) {
            rows.push(
                Row::new(vec![Cell::from(Span::raw(skill.name))]));
            rows.push(
                Row::new(vec![Cell::from(Span::raw(skill.description))]));
        }
    }

    let char_skill_table = Table::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Talanger")
                .border_type(BorderType::Plain),
        )
        .widths(&[Constraint::Percentage(100)]);

    char_skill_table
}

fn read_skill_db() -> Result<Vec<Skill>, Error > {
    let db_content = fs::read_to_string(SKILL_DB_PATH)?;
    let parsed: Vec<Skill> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_character_db() -> Result<Vec<Character>, Error > {
    let db_content = fs::read_to_string(CHARACTER_DB_PATH)?;
    let parsed: Vec<Character> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}