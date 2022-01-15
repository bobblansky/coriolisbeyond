use std::fs;
use std::io;
use std::thread;
use std::sync::mpsc;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend,CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap, Clear
    },
    Terminal, Frame,
};
mod lore;
mod banner;
use lore::LORE;
use banner::BANNER;
const  SKILL_DB: &str = "./data/skills.json";
const CHARACTER_DB: &str = "./data/character.json";
const WEAPON_DB: &str = "./data/weapons.json";
const ITEM_DB: &str = "./data/items.json";
const ARMOR_DB: &str = "./data/armor.json";

#[cfg(test)]
#[test]
fn test_path() {
    use std::path::Path;
    const NUMPATHS: usize = 5;
    let paths: [&str;NUMPATHS] = [SKILL_DB, CHARACTER_DB, WEAPON_DB, ITEM_DB, ARMOR_DB];
    paths.map(|p| assert_eq!(Path::new(p).exists(), true));
}

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
struct Item {
    id: usize,
    name: String,
    description: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Appearance {
    face: String,
    clothing: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Armor {
    id: usize,
    name: String,
    rating: u8,
    addons: String,
    tech: String,
    comment: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Weapon {
    id: usize,
    namn: String,
    bonus: u8,
    init: u8,
    skada: u8,
    krit: u8,
    räckvidd: String,
    övrigt: String,
    kostnad: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Kvalificerade {
    befäl: u8,
    datadjinn: u8,
    horistonens_kultur: u8,
    medikrugi: u8,
    mystiska_krafter: u8,
    pilot: u8,
    teknologi: u8,
    vetenskap: u8,
}

#[derive(Serialize, Deserialize, Clone)]
struct Allmanna {
    kraftprov: u8,
    manipulera: u8,
    närkamp: u8,
    rörlighet: u8,
    skjutvapen: u8,
    smyga: u8,
    spaning: u8,
    överlevnad: u8,
}

#[derive(Serialize, Deserialize, Clone)]
struct Fardigheter {
    allmanna: Allmanna,
    kvalificerade: Kvalificerade,
}

#[derive(Serialize, Deserialize, Clone)]
struct Grundegenskaper {
    styrka: u8,
    kyla: u8,
    skärpa: u8,
    känsla: u8,
}

#[derive(Serialize, Deserialize, Clone)]
struct Character {
    id: usize,
    name: String,
    experience: u8,
    class: String,
    ship_position: String,
    problem: String,
    icon: String,
    background: String,
    upbringing: String,
    group_concept: String,
    skill_ids: Vec<usize>,
    weapon_ids: Vec<usize>,
    armor_ids: Vec<usize>,
    gear_ids: Vec<usize>,
    birr: u32,
    appearance: Appearance,
    grundegenskaper: Grundegenskaper,
    fardigheter: Fardigheter,
}

#[derive(Serialize, Deserialize, Clone)]
struct Skill {
    id: usize,
    name: String,
    description: String,
    category: String,
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Character,
    Skills,
    Items,
    Lore
}
impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Character => 1,
            MenuItem::Skills => 2,
            MenuItem::Items => 3,
            MenuItem::Lore => 4
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

    let mut show_skill_popup = false;
    let mut select_skill_list = false;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Hem", "Karaktärer", "Talanger", "Utrustning", "Lore", "Avsluta"];
    let mut active_menu_item = MenuItem::Home;
    let mut list_state = ListState::default();
    let mut list_state_skills = ListState::default();
    list_state.select(Some(0));
    let mut skillcounter = 0;
    let mut charcounter = 0;
    let mut itemcounter = 0;
    let mut homecounter = 0;
    let mut scroll = 1;
    let mut current_menu: MenuItem = MenuItem::Home;
    list_state_skills.select(Some(0));

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
                .style(Style::default().fg(Color::DarkGray))
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
            
            // state machine implementation?
            let refresh_needed : bool = current_menu == active_menu_item;
            match active_menu_item {
                MenuItem::Home => {
                    homecounter = homecounter + 1;
                    let home_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(30),
                            Constraint::Ratio(3,1),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[1]);
                    let (banner_text, home_text) = render_home();
                    rect.render_widget(banner_text, home_chunks[0]);
                    rect.render_widget(home_text, home_chunks[1]);
                },
                MenuItem::Character => {
                    if refresh_needed {
                    //derbug counter
                    charcounter = charcounter + 1;
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
                        .constraints([
                            //Max X där X är antal färdigheter + 2 (top/botten -linjer värda 1)
                            Constraint::Min(19), 
                            Constraint::Percentage(20),
                            Constraint::Percentage(20), 
                            Constraint::Percentage(25)].as_ref(),
                        )
                        .split(character_chunks[1]);
                    let character_chunk = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(50),
                            Constraint::Percentage(15), 
                            Constraint::Percentage(35)].as_ref(),
                        )
                        .split(inside_chunks[0]);
                    let talent_gear_chunk = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(50),
                            Constraint::Percentage(50)].as_ref(),
                    )
                    .split(inside_chunks[1]);
                    //get the character and render
                    //left => name
                    //right list character info from json
                    let (left, right, grundegenskaper, fardigheter, char_skills_ids, weapon_ids, gear_ids, armor_ids) = render_character(&mut list_state);
                    let (left1, right2) = render_char_skills(&mut list_state_skills, &char_skills_ids); // char_skills
                    let weapons = render_character_weapons(weapon_ids);
                    let armor = render_character_armor(armor_ids);
                    let items = render_character_items(gear_ids);
                    rect.render_widget(items, talent_gear_chunk[1]);
                    if select_skill_list{
                        rect.render_widget(left, character_chunks[0]);
                        rect.render_stateful_widget(left1, talent_gear_chunk[0], &mut list_state_skills);
                    }
                    else{
                        rect.render_stateful_widget(left, character_chunks[0], &mut list_state);
                        rect.render_widget(left1, talent_gear_chunk[0]);
                    }
                    rect.render_widget(right, character_chunk[0]);
                    rect.render_widget(grundegenskaper, character_chunk[1]);
                    rect.render_widget(fardigheter, character_chunk[2]);
                    rect.render_widget(armor, inside_chunks[2]);
                    rect.render_widget(weapons, inside_chunks[3]);
                    if show_skill_popup{
                        render_popup(rect, &mut list_state_skills, char_skills_ids)
                    }
                    }

                },
                //debug
                MenuItem::Skills => {
                    //derbug counter
                    skillcounter = skillcounter + 1;
                    let skill_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_skills(&mut list_state);

                    rect.render_stateful_widget(left, skill_chunks[0], &mut list_state);
                    rect.render_widget(right, skill_chunks[1]);
                },
                MenuItem::Items => {
                    //derbug counter
                    itemcounter = itemcounter + 1;
                    let item_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_items(&mut list_state);
                    rect.render_stateful_widget(left, item_chunks[0], &mut list_state);
                    rect.render_widget(right, item_chunks[1]);
                },
                MenuItem::Lore => {
                    let lore_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [Constraint::Max(1), Constraint::Percentage(95)].as_ref(),
                        )
                        .horizontal_margin(10)
                        .vertical_margin(1)
                        .split(chunks[1]);
                    let lore_block = Block::default()
                        .borders(Borders::LEFT | Borders::RIGHT)
                        .style(Style::default().fg(Color::Cyan));


                    let lore_text = Paragraph::new(LORE).style(Style::default().add_modifier(Modifier::BOLD).fg(Color::White)).scroll((scroll,1)).block(lore_block);
                    rect.render_widget(lore_text, lore_chunks[1]);
                }
            }
            rect.render_widget(copyright, chunks[2]);
        })?;
        
        current_menu = active_menu_item; 
        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('a') => {
                    terminal.clear()?;
                    let mut stdout = io::stdout();
                    execute!(stdout, LeaveAlternateScreen)?;
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('h') => active_menu_item = MenuItem::Home,
                KeyCode::Char('k') => active_menu_item = MenuItem::Character,
                KeyCode::Char('t') => active_menu_item = MenuItem::Skills,
                KeyCode::Char('u') => active_menu_item = MenuItem::Items,
                KeyCode::Char('l') => active_menu_item = MenuItem::Lore,
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
                        if select_skill_list {
                            if let Some(selected) = list_state_skills.selected() {
                                let amount_characters = read_character_db().expect("can fetch list").len();
                                if selected >0 {
                                    list_state_skills.select(Some(0));
                                } else {
                                    list_state_skills.select(Some(selected + 1));
                                }
                            }
                        }
                        else{
                            if let Some(selected) = list_state.selected() {
                                let amount_characters = read_character_db().expect("can fetch list").len();
                                if selected >= amount_characters - 1 {
                                    list_state.select(Some(0));
                                } else {
                                    list_state.select(Some(selected + 1));
                                }
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Items {
                        if let Some(selected) = list_state.selected() {
                            let amount_items = read_item_db().expect("can fetch list").len();
                            if selected >= amount_items - 1 {
                                list_state.select(Some(0));
                            } else {
                                list_state.select(Some(selected + 1));
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Lore {
                        scroll = scroll + 1;
                        if scroll >= 15 {
                            scroll = 15;
                        }
                    }
                }
                KeyCode::Up => {
                    if active_menu_item == MenuItem::Items {
                        if let Some(selected) = list_state.selected() {
                            let amount_items = read_item_db().expect("can fetch list").len();
                            if selected >= amount_items - 1 {
                                list_state.select(Some(0));
                            } else {
                                list_state.select(Some(selected - 1));
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Skills {
                        if let Some(selected) = list_state.selected() {
                            let amount_skills = read_character_db().expect("can fetch list").len();
                            if selected >= amount_skills {
                                list_state.select(Some(selected - 1));
                            } else {
                                list_state.select(Some(amount_skills - 1));
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Character {
                        if select_skill_list {
                            if let Some(selected) = list_state_skills.selected() {
                                let amount_skills = read_character_db().expect("can fetch list").len();
                                if selected >0 {
                                    list_state_skills.select(Some(selected - 1));
                                } else {
                                    list_state_skills.select(Some(amount_skills - 1));
                                }
                            }
                        }
                        else {
                            if let Some(selected) = list_state.selected() {
                                let amount_characters = read_character_db().expect("can fetch list").len();
                                if selected >= amount_characters - 1 {
                                    list_state.select(Some(selected - 1));
                                } else {
                                    list_state.select(Some(amount_characters - 1));
                                }
                            }
                        }
                    }
                    if active_menu_item == MenuItem::Lore {
                        scroll = scroll - 1;
                        if scroll <= 0 {
                            scroll = 1;
                        }
                    }
                }
                KeyCode::Right => {
                    if active_menu_item == MenuItem::Character {
                        select_skill_list = true;
                    }
                }
                KeyCode::Left => {
                    if active_menu_item == MenuItem::Character {
                        select_skill_list = false;
                    }
                }
                KeyCode::Enter => {
                    if active_menu_item == MenuItem::Character {
                        show_skill_popup = !show_skill_popup;
                    }
                    if active_menu_item == MenuItem::Items {
                        if let Some(selected) = list_state.selected() {
                            let amount_items = read_item_db().expect("can fetch list").len();
                            if selected >= amount_items - 1 {
                                list_state.select(Some(0));
                            } else {
                                list_state.select(Some(selected - 1));
                            }
                        }
                    }
                }
                KeyCode::Right => {
                    if active_menu_item == MenuItem::Character {
                        select_skill_list = true;
                    }
                }
                KeyCode::Left => {
                    if active_menu_item == MenuItem::Character {
                        select_skill_list = false;
                    }
                }
                KeyCode::Enter => {
                    if active_menu_item == MenuItem::Character {
                        show_skill_popup = !show_skill_popup;
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }
    println!("Number of skill db reads: {:?}", skillcounter);
    println!("Number of character db reads: {:?}", charcounter);
    println!("Number of item db reads: {:?}", itemcounter);
    Ok(())
}

fn render_popup<B: Backend>(rect: &mut Frame<B>, list_state: &ListState, char_skills: Vec<usize>){
    let skills = read_skill_db().expect("can fetch skill list");
    let mut skill_char: Vec<_> = Vec::new();
    for skill in skills{
        if char_skills.contains(&skill.id){
            skill_char.push(skill);
        }
    }

    let selected_skill = skill_char
    .get(
        list_state
            .selected()
            .expect("there is always a selected skill"),
    )
    .expect("exists")
    .clone();

    let size = rect.size();
    let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let span = Span::styled(selected_skill.name, style);
    let block = Block::default().title(span).borders(Borders::ALL);
    let pop_up = Paragraph::new(selected_skill.description).wrap(Wrap{trim:true}).block(block);

    let area = centered_rect(64, 36, size);
    rect.render_widget(Clear, area);
    rect.render_widget(pop_up, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn render_home<'a>() -> (Paragraph<'a>, Paragraph<'a>) {
    let home = Paragraph::new("
        'Med Zenit kom den nya eran – och Horisonten blomstrade återigen. 
        Tre dussin stjärnsystem, sammanbundna av ödet och Ikonerna, vandrade tillsammans mot en ljusare framtid. 
        Men med Emissariernas ankomst led den lyckliga tiden mot sitt slut. Och mörkret mellan stjärnorna anades åter.'\n
        IKONERNAS RIKE – En historisk översikt av Tredje horisonten av Kaldana Mourir")
        .wrap(Wrap {trim:true})
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
    let banner_para = Paragraph::new(BANNER).alignment(Alignment::Center).style(Style::default().fg(Color::Cyan));

    (banner_para, home)
}

fn render_character<'a>(list_state: &mut ListState) -> (List<'a>, Table<'a>, Table<'a>, Table<'a>, Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>) {
    let character = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Karaktärer")
        .border_type(BorderType::Plain);

    let character_list = read_character_db().expect("can fetch skill list");
    //Checks index boundary, sets zero if out of bounds.
    if list_state.selected().unwrap() > character_list.len() {
        list_state.select(Some(0));
    }

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

    let grundegenskaper_table = Table::new(vec![
        Row::new(vec![Cell::from("Styrka"), Cell::from(Span::raw(selected_character.grundegenskaper.styrka.to_string()))]),
        Row::new(vec![Cell::from("Kyla"), Cell::from(Span::raw(selected_character.grundegenskaper.kyla.to_string()))]),
        Row::new(vec![Cell::from("Skärpa"), Cell::from(Span::raw(selected_character.grundegenskaper.skärpa.to_string()))]),
        Row::new(vec![Cell::from("Känsla"), Cell::from(Span::raw(selected_character.grundegenskaper.känsla.to_string()))]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Grundegenskaper")
            .border_type(BorderType::Plain),
    )
    .widths(&[Constraint::Percentage(80), Constraint::Percentage(10)]);

    let fardigheter_table = Table::new(vec![
        Row::new(vec![
            Cell::from("Kraftprov (STY)"),
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.kraftprov.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.styrka + selected_character.fardigheter.allmanna.kraftprov).to_string()))]),
        Row::new(vec![
            Cell::from("Manipulera (KNS)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.manipulera.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.känsla + selected_character.fardigheter.allmanna.manipulera).to_string()))]),
        Row::new(vec![
            Cell::from("Närkamp (STY)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.närkamp.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.styrka + selected_character.fardigheter.allmanna.närkamp).to_string()))]),
        Row::new(vec![
            Cell::from("Rörlighet (KYL)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.rörlighet.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.kyla + selected_character.fardigheter.allmanna.rörlighet).to_string()))]),
        Row::new(vec![
            Cell::from("Skjutvapen (KYL)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.skjutvapen.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.kyla + selected_character.fardigheter.allmanna.skjutvapen).to_string()))]),
        Row::new(vec![
            Cell::from("Smyga (KYL)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.smyga.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.kyla + selected_character.fardigheter.allmanna.smyga).to_string()))]),
        Row::new(vec![
            Cell::from("Spaning (SKP)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.spaning.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.skärpa + selected_character.fardigheter.allmanna.spaning).to_string()))]),
        Row::new(vec![
            Cell::from("Överlevnad (SKP)"), 
            Cell::from(Span::raw(selected_character.fardigheter.allmanna.överlevnad.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                (selected_character.grundegenskaper.skärpa + selected_character.fardigheter.allmanna.överlevnad).to_string()))]),
        Row::new(vec![Cell::from("- Kvalificerade -")]),
        Row::new(vec![
            Cell::from("Befäl (KNS)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.befäl.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.befäl > 0 {
                    (selected_character.grundegenskaper.känsla + selected_character.fardigheter.kvalificerade.befäl).to_string()
                }
                else { String::from("0") }
                ))]),
        Row::new(vec![
            Cell::from("Datadjinn (SKP)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.datadjinn.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.datadjinn > 0 {
                    (selected_character.grundegenskaper.skärpa + selected_character.fardigheter.kvalificerade.datadjinn).to_string()
                }
                else { String::from("0") }
                ))]),
        Row::new(vec![
            Cell::from("Horisontens kultur (KNS)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.horistonens_kultur.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.horistonens_kultur > 0 {
                    (selected_character.grundegenskaper.känsla + selected_character.fardigheter.kvalificerade.horistonens_kultur).to_string()
                }
                else { String::from("0") }
                ))]),
        Row::new(vec![
            Cell::from("Medikurgi (SKP)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.medikrugi.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.medikrugi > 0 {
                    (selected_character.grundegenskaper.skärpa + selected_character.fardigheter.kvalificerade.medikrugi).to_string()
                }
                else { String::from("0") }
                ))]),
        Row::new(vec![
            Cell::from("Mystiska krafter (KNS)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.mystiska_krafter.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.mystiska_krafter > 0 {
                    (selected_character.grundegenskaper.känsla + selected_character.fardigheter.kvalificerade.mystiska_krafter).to_string()
                } 
                else { String::from("0") }
            ))]),
        Row::new(vec![
            Cell::from("Pilot (KYL)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.pilot.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.pilot > 0 {
                    (selected_character.grundegenskaper.kyla + selected_character.fardigheter.kvalificerade.pilot).to_string()
                }
                else { String::from("0") }
            ))]),
        Row::new(vec![
            Cell::from("Teknologi (SKP)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.teknologi.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.teknologi > 0 {
                    (selected_character.grundegenskaper.skärpa + selected_character.fardigheter.kvalificerade.teknologi).to_string()
                }
                else { String::from("0") }
            ))]),
        Row::new(vec![
            Cell::from("Vetenskap (SKP)"), 
            Cell::from(Span::raw(selected_character.fardigheter.kvalificerade.vetenskap.to_string())),
            Cell::from(Span::raw(" => ")),
            Cell::from(Span::raw(
                if selected_character.fardigheter.kvalificerade.vetenskap > 0 {
                    (selected_character.grundegenskaper.skärpa + selected_character.fardigheter.kvalificerade.vetenskap).to_string()
                } 
                else { String::from("0") }
            ))]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Färdigheter")
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(30),
        Constraint::Percentage(2),
        Constraint::Percentage(5),
        Constraint::Percentage(5)]);

    let character_detail = Table::new(vec![
        Row::new(vec![
            Cell::from(Span::raw("Klass: ")),
            Cell::from(Span::raw(selected_character.class))]),
        Row::new(vec![
            Cell::from(Span::raw("Position: ")),
            Cell::from(Span::raw(selected_character.ship_position))]),
        Row::new(vec![
            Cell::from(Span::raw("Bakgrund: ")),
            Cell::from(Span::raw(selected_character.background))]),
        Row::new(vec![
            Cell::from(Span::raw("Uppväxt: ")),
            Cell::from(Span::raw(selected_character.upbringing))]),
        Row::new(vec![
            Cell::from(Span::raw("Gruppkoncept: ")),
            Cell::from(Span::raw(selected_character.group_concept))]),
        Row::new(vec![
            Cell::from(Span::raw("Ikon: ")),
            Cell::from(Span::raw(selected_character.icon))]),
        Row::new(vec![
            Cell::from(Span::raw("Problem: ")),
            Cell::from(Span::raw(selected_character.problem))]),
        Row::new(vec![
            Cell::from(Span::raw("Birr: ")),
            Cell::from(Span::raw(selected_character.birr.to_string()))]),
        Row::new(vec![Cell::from(Span::raw("\n\n"))]),
        Row::new(vec![Cell::from(Span::raw("Utseende\n "))]),
        Row::new(vec![
            Cell::from(Span::raw("Ansikte: ")),
            Cell::from(Span::raw(selected_character.appearance.face))]),
        Row::new(vec![
            Cell::from(Span::raw("Kläder: ")),
            Cell::from(Span::raw(selected_character.appearance.clothing))]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title(selected_character.name)
            .border_type(BorderType::Plain),
    )
    .widths(&[Constraint::Percentage(15), Constraint::Percentage(80)]);


    (list, 
    character_detail, 
    grundegenskaper_table,
    fardigheter_table, 
    selected_character.skill_ids, 
    selected_character.weapon_ids,
    selected_character.gear_ids,
    selected_character.armor_ids,
    )
}

fn render_skills<'a>(list_state: &mut ListState) -> (List<'a>, Paragraph<'a>) {
    let skill_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Talanger")
        .border_type(BorderType::Plain);

    let skill_list = read_skill_db().expect("can fetch skill list");
    //Checks index boundary, sets zero if out of bounds.
    if list_state.selected().unwrap() > skill_list.len() {
        list_state.select(Some(0));
    }
    
    let mut category  = String::from("");
    let items: Vec<_> = skill_list
        .iter()
        .map(|skill| {
            if category != skill.category {
                category = skill.category.clone();     
                ListItem::new(Spans::from(vec![
                    Span::styled(skill.category.clone(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                )]))
            } else {
                ListItem::new(Spans::from(vec![Span::styled(
                    skill.name.clone(),
                    Style::default(),
                )]))
            }
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

    let list = List::new(items).block(skill_block).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let skill_detail = Paragraph::new(selected_skill.description)
        .wrap(Wrap{trim:true})
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title(selected_skill.name)
                .border_type(BorderType::Plain),
        );

    (list, skill_detail)
}

fn render_char_skills<'a>(list_state: &mut ListState, char_skills: &Vec<usize>) -> (List<'a>, Paragraph<'a>) {
    let skills = Block::default()
    .borders(Borders::ALL)
    .style(Style::default().fg(Color::White))
    .title("Talanger")
    .border_type(BorderType::Plain);

    let skill_list = read_skill_db().expect("can fetch skill list");
    let mut skill_char: Vec<_> = Vec::new();
    
    for skill in  skill_list{
        if char_skills.contains(&skill.id){
            skill_char.push(skill);
        }
    }
    let skill_list_len = skill_char.len()-1;
    let items: Vec<_> = skill_char
        .iter()
        .map(|skill| {
            ListItem::new(Spans::from(vec![Span::styled(
                skill.name.clone(),
                Style::default(),
            )]))
        })
        .collect();
    //Checks index boundary, sets zero if out of bounds.
    if list_state.selected().unwrap() > skill_list_len {
        list_state.select(Some(0));
    }
    let selected_skill = skill_char
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

fn render_character_skills<'a>(char_skills: Vec<usize>) -> Table<'a> {
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

fn render_character_items<'a>(char_skills: Vec<usize>) -> List<'a> {
    let item_list = read_item_db().expect("can fetch item list");
    let mut items: Vec<_> = Vec::new();
    let item_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Utrustning")
        .border_type(BorderType::Plain);

    for skill in item_list {
        if char_skills.contains(&skill.id) {
            items.push(
                ListItem::new(Spans::from(vec![
                    (Span::raw(skill.name.clone()))
                ]))
            );
        }
    }

    let list = List::new(items).block(item_block);
    
    list
}

fn render_items<'a>(list_state: &mut ListState) -> (List<'a>, Paragraph<'a>) {
    let item_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Utrustning")
        .border_type(BorderType::Plain);

    let item_list = read_item_db().expect("can fetch item list");
    //Checks index boundary, sets zero if out of bounds.
    if list_state.selected().unwrap() > item_list.len() {
        list_state.select(Some(0));
    }

    let items: Vec<_> = item_list
        .iter()
        .map(|i| {
            ListItem::new(Spans::from(vec![Span::styled(
                i.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_item = item_list
        .get(
            list_state
                .selected()
                .expect("there is always a selected item"),
        )
        .expect("")
        .clone();

    let list = List::new(items).block(item_block).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let item_detail = Paragraph::new(selected_item.description)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title(selected_item.name)
                .border_type(BorderType::Plain),
        );

    (list, item_detail)
}

fn render_character_weapons<'a>(char_weapons: Vec<usize>) -> Table<'a> {
    let weapon_list = read_weapon_db().expect("can fetch weapon list");

    let mut rows: Vec<Row> = Vec::new();

    for weapon in weapon_list {
        if char_weapons.contains(&weapon.id) {
            rows.push(
                Row::new(vec![
                    Cell::from(Span::raw(weapon.namn)),
                    Cell::from(Span::raw(weapon.bonus.to_string())),
                    Cell::from(Span::raw(weapon.init.to_string())),
                    Cell::from(Span::raw(weapon.skada.to_string())),
                    Cell::from(Span::raw(weapon.krit.to_string())),
                    Cell::from(Span::raw(weapon.räckvidd)),
                    Cell::from(Span::raw(weapon.övrigt))
                ])
            );
        }
    }
    let normal_style = Style::default().bg(Color::DarkGray);
    let header_cells = ["Weapon", "Bonus", "Init", "Skada", "Krit", "Räckvidd", "Övrigt"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::White)));
    let header = Row::new(header_cells)
        .style(normal_style);
    let char_weapon_table = Table::new(rows)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                        .title("Vapen")
                        .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Min(20),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(20),
            Constraint::Percentage(20)
            ]);

    char_weapon_table
}

fn render_character_armor<'a>(char_armor: Vec<usize>) -> Table<'a> {
    let armor_list = read_armor_db().expect("can fetch armor list");

    let mut rows: Vec<Row> = Vec::new();

    for armor in armor_list {
        if char_armor.contains(&armor.id) {
            rows.push(
                Row::new(vec![
                    Cell::from(Span::raw(armor.name)),
                    Cell::from(Span::raw(armor.rating.to_string())),
                    Cell::from(Span::raw(armor.comment)),
                ])
            );
        }
    }
    let normal_style = Style::default().bg(Color::DarkGray);
    let header_cells = ["Rustning", "Skydd", "Övrigt"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::White)));
    let header = Row::new(header_cells)
        .style(normal_style);
    let char_armor_table = Table::new(rows)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                        .title("Rustning")
                        .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Min(20),
            Constraint::Percentage(10),
            Constraint::Percentage(50),
            ]);

    char_armor_table
}

fn read_skill_db() -> Result<Vec<Skill>, Error > {
    let db_content = fs::read_to_string(SKILL_DB)?;
    let parsed: Vec<Skill> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_character_db() -> Result<Vec<Character>, Error > {
    let db_content = fs::read_to_string(CHARACTER_DB)?;
    let parsed: Vec<Character> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_weapon_db() -> Result<Vec<Weapon>, Error > {
    let db_content = fs::read_to_string(WEAPON_DB)?;
    let parsed: Vec<Weapon> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_item_db() -> Result<Vec<Item>, Error > {
    let db_content = fs::read_to_string(ITEM_DB)?;
    let parsed: Vec<Item> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn read_armor_db() -> Result<Vec<Armor>, Error > {
    let db_content = fs::read_to_string(ARMOR_DB)?;
    let parsed: Vec<Armor> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

