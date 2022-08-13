use crate::utils::sql::fetch::get_topics;
use rusqlite::Connection;
use crossterm::event::KeyCode;

use crate::logic::{
    review::ReviewList,
    browse::Browse,
    add_card::{NewCards, TextSelect},
};
use crate::events::{
    review::review_event,
    browse::browse_event,
    add_card::add_card_event,
    import::main_port,
};







pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}


impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0}
    }
    pub fn next(&mut self) {
        if self.index < self.titles.len() - 1 {
            self.index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }
}







pub struct App<'a> {
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub conn: Connection,
    pub prev_key: KeyCode,
    pub review: ReviewList,
    pub add_card: NewCards,
    pub browse: Browse,
    
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let conn = Connection::open("dbflash.db").expect("Failed to connect to database.");
        let revlist = ReviewList::new(&conn);
        let browse = Browse::new(&conn);
        let addcards =  NewCards::new(&conn);
        let app = App {
            should_quit: false,
            tabs: TabsState::new(vec!["Review", "Add card", "Browse cards 🦀", "import"]),
            conn: conn,
            prev_key: KeyCode::Null,
            review: revlist,
            add_card: addcards,
            browse: browse,
        };
        //app.add_card.topics.dfs();
        app
    }


    pub fn on_right(&mut self) {
        if self.add_card.card.selection == TextSelect::Question(false) || self.add_card.card.selection == TextSelect::Answer(false){
            self.tabs.next();
        }
    }

    pub fn on_left(&mut self) {
        if self.add_card.card.selection == TextSelect::Question(false) || self.add_card.card.selection == TextSelect::Answer(false){
            self.tabs.previous();
        }
    }




    pub fn on_tick(&mut self) {}

    
    pub fn handle_key(&mut self, key: KeyCode) {
        let keyclone = key.clone();
        match self.tabs.index {
            0 => review_event(self, key),
            1 => add_card_event(self, key),
            2 => browse_event(self, key),
            3 => main_port(self, key),
            _ => {},
        }
        self.prev_key = keyclone;
    }



}

