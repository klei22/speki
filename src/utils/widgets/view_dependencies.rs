
use crate::utils::statelist::StatefulList;
use crate::utils::widgets::list::StraitList;
use tui::widgets::ListItem;
use tui::widgets::ListState;
use tui::widgets::List;
use crate::utils::sql::fetch::fetch_card;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    widgets::{
        Block, Borders},
    Frame,
};

type Dependency = crate::utils::CardInList;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use crate::utils::widgets::list::list_widget;
use tui::text::Spans;
use tui::style::Modifier;

pub fn view_dependencies<B>(f: &mut Frame<B>, id: u32, conn: &Arc<Mutex<Connection>>, area: Rect, selected: bool)
where
    B: Backend,
{
    let thecard = fetch_card(&conn, id);
    let dep_ids = &thecard.dependencies;
    let mut dependent_vec = Vec::<Dependency>::new();

    for id in dep_ids{
        dependent_vec.push(
            Dependency{
                question: fetch_card(&conn, *id).question,
                id: *id,
            }
        );
    }
    let statelist = StatefulList::with_items(dependent_vec);
    list_widget(f, &statelist, area, selected, "Dependencies".to_string());
}



impl<T> StraitList<T> for StatefulList<Dependency>{

    fn state(&self) -> ListState {
        self.state.clone()
    }

    fn generate_list_items(&self, selected: bool, title: String) -> List{
    let bordercolor = if selected {Color::Red} else {Color::White};
    let style = Style::default().fg(bordercolor);

    let items: Vec<ListItem> = self.items.iter().map(|dependency| {
        let lines = vec![Spans::from(dependency.question.clone())];
        ListItem::new(lines).style(Style::default())
    }).collect();
    
    let items = List::new(items)
        .block(
            Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title)
            );
    
    if selected{
    items
        .highlight_style(
            Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )}
    else {items}
    }
}
