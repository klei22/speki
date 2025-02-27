
use std::time::{UNIX_EPOCH, SystemTime};
use crate::utils::{
    card::{Card, RecallGrade},
    sql::{fetch::{load_cards, get_strength}, update::double_inc_skip_duration},
};

use rusqlite::Connection;
use crate::utils::aliases::*;
use crate::utils::widgets::textinput::Field;
use crate::utils::incread::IncRead;
use rand::prelude::*;
use crate::utils::sql::update::update_inc_active;
use crate::utils::widgets::cardrater::CardRater;
use crate::utils::widgets::load_cards::MediaContents;
use std::sync::{Arc, Mutex};



pub enum ReviewSelection{
    Question,
    Answer,
    Dependencies,
    Dependents,
    RevealButton,
    CardRater,
}


pub struct CardReview{
    pub id: CardID,
    pub question: Field,
    pub answer: Field,
    pub reveal: bool,
    pub selection: ReviewSelection,
    pub cardrater: CardRater,
    pub media: MediaContents,
}


pub struct UnfCard{
    pub id: CardID,
    pub question: Field,
    pub answer: Field,
    pub selection: UnfSelection,
}

pub enum UnfSelection{
    Question,
    Answer,
    Dependencies,
    Dependents,
}


pub struct IncMode{
    pub id: IncID,
    pub source: IncRead,
    pub selection: IncSelection,
}

pub enum IncSelection{
    Source,
    Clozes,
    Extracts,
}


pub enum ReviewMode{
    Review(CardReview),
    Pending(CardReview),
    Unfinished(UnfCard),
    IncRead(IncMode),
    Done,
}


pub struct ForReview{
    pub review_cards:     Vec<CardID>,
    pub unfinished_cards: Vec<CardID>,
    pub pending_cards:    Vec<CardID>,
    pub active_increads:  Vec<IncID>,
}


impl ForReview{
    pub fn new(conn: &Arc<Mutex<Connection>>)-> Self{
        crate::utils::interval::calc_strength(conn);
        let thecards = load_cards(&conn).unwrap();
        let mut review_cards     = Vec::<CardID>::new();
        let mut unfinished_cards = Vec::<CardID>::new();
        let mut pending_cards    = Vec::<CardID>::new();

        let active_increads  = load_active_inc(&conn).unwrap();

        for card in thecards{
            if !card.resolved || card.suspended {continue}

            if card.is_complete(){
                if get_strength(&conn, card.id).unwrap() < 0.9{
                    review_cards.push(card.id);
                }
            } else if card.is_unfinished(){
                let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32;
                if current_time - card.skiptime(conn) > card.skipduration(conn) * 84_600{
                    unfinished_cards.push(card.id);
                }
            } else if card.is_pending(){
                pending_cards.push(card.id);
            }
        }

        unfinished_cards.shuffle(&mut thread_rng());
        review_cards.shuffle(&mut thread_rng());

        ForReview{
            review_cards,
            unfinished_cards,
            pending_cards,
            active_increads,
        }
    }
}


pub struct StartQty{
    pub fin_qty: u16,
    pub unf_qty: u16,
    pub pending_qty: u16,
    pub inc_qty: u16,
}

impl StartQty{
    pub fn new(for_review: &ForReview) -> Self{
        let fin_qty = for_review.review_cards.len() as u16;
        let unf_qty = for_review.unfinished_cards.len() as u16;
        let pending_qty = for_review.pending_cards.len() as u16;
        let inc_qty = for_review.active_increads.len() as u16;

        StartQty{
            fin_qty,
            unf_qty,
            pending_qty,
            inc_qty
        }
    }
}



pub struct ReviewList{
    pub title: String,
    pub mode: ReviewMode,
    pub for_review: ForReview,
    pub start_qty: StartQty,
    pub automode: bool,
}

use crate::utils::sql::fetch::{fetch_card, load_active_inc, fetch_media};


impl ReviewList {


    pub fn new(conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle)->ReviewList{
        //interval::calc_strength(conn);

        let mode = ReviewMode::Done;
        let for_review = ForReview::new(conn);
        let start_qty = StartQty::new(&for_review);

        let mut myself = ReviewList{
            title: String::from("review!"),
            mode,
            for_review,
            start_qty,
            automode: true,
        };
        myself.random_mode(conn, handle);
        myself
        }

    // randomly choose a mode between active, unfinished and inc read, if theyre all done, 
    // start with pending cards, if theyre all done, declare nothing left to review
    pub fn random_mode(&mut self, conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle){
        let act: u32 = self.for_review.review_cards.len()     as u32;
        let unf: u32 = self.for_review.unfinished_cards.len() as u32 + act;
        let inc: u32 = self.for_review.active_increads.len()  as u32 + unf;

        let pending_qty = self.for_review.pending_cards.len() as u32;
        if inc == 0{
            if pending_qty > 0{
                self.new_pending_mode(conn, handle);
            } else {
                self.mode = ReviewMode::Done;
            }
            return;
        }

        let mut rng = rand::thread_rng();
        let rand = rng.gen_range(0..inc);

        if rand < act {
            self.new_review_mode(conn, handle);
        } else if rand < unf {
            self.new_unfinished_mode(conn, handle);
        } else if rand < inc {
            self.new_inc_mode(conn);
        } else{
            panic!();
        };
    }



    pub fn new_inc_mode(&mut self, conn: &Arc<Mutex<Connection>>){
        let id = self.for_review.active_increads.remove(0);
        let selection = IncSelection::Source;
        let source = IncRead::new(conn, id);
        let inc = IncMode{
            id,
            source,
            selection,
        };

        self.mode = ReviewMode::IncRead(inc);
    }
    pub fn new_unfinished_mode(&mut self, conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle){
        let id = self.for_review.unfinished_cards.remove(0);
        Card::play_frontaudio(conn, id, handle);
        let selection = UnfSelection::Question;
        let mut question = Field::new();
        let mut answer   = Field::new();
        let card = fetch_card(&conn, id);
        question.replace_text(card.question);
        answer.replace_text(card.answer);
        let unfcard = UnfCard{
            id,
            question,
            answer,
            selection,
        };
        self.mode = ReviewMode::Unfinished(unfcard);
    }

    pub fn new_pending_mode(&mut self, conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle){
        let id = self.for_review.pending_cards.remove(0);
        Card::play_frontaudio(conn, id, handle);
        let reveal = false;
        let selection = ReviewSelection::RevealButton;
        let mut question = Field::new();
        let mut answer = Field::new();
        let card = fetch_card(&conn, id);
        question.replace_text(card.question);
        answer.replace_text(card.answer);
        let cardrater = CardRater::new();
        let media = fetch_media(&conn, id);
        let cardreview = CardReview{
            id,
            question,
            answer,
            reveal,
            selection,
            cardrater,
            media,
        };

        self.mode = ReviewMode::Pending(cardreview);
    }
    pub fn new_review_mode(&mut self, conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle ){
        let id = self.for_review.review_cards.remove(0);
        Card::play_frontaudio(conn, id, handle);
        let reveal = false;
        let selection = ReviewSelection::RevealButton;
        let mut question = Field::new();
        let mut answer = Field::new();
        let card = fetch_card(&conn, id);
        question.replace_text(card.question);
        answer.replace_text(card.answer);
        let cardrater = CardRater::new();
        let media = fetch_media(&conn, id);
        let cardreview = CardReview{
            id,
            question,
            answer,
            reveal,
            selection,
            cardrater,
            media,
        };

        self.mode = ReviewMode::Review(cardreview);
    }

    pub fn inc_next(&mut self, conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle, id: IncID ){
        self.random_mode(conn, handle);
        double_inc_skip_duration(conn, id).unwrap();
    }
    pub fn inc_done(&mut self, id: IncID, conn: &Arc<Mutex<Connection>>, handle: &rodio::OutputStreamHandle){
        let active = false;
        update_inc_active(&conn, id, active).unwrap();
        self.random_mode(conn, handle);

    }



    pub fn new_review(&mut self, conn: &Arc<Mutex<Connection>>, id: CardID, recallgrade: RecallGrade, handle: &rodio::OutputStreamHandle ){
        Card::new_review(conn, id, recallgrade);
        self.random_mode(conn, handle);
    }
}



