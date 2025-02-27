use rusqlite::{Connection,Row, Result};
use crate::utils::card::{Card, RecallGrade, Review}; //, Topic, Review}
use crate::utils::widgets::topics::Topic;
use crate::utils::aliases::*;
use crate::utils::widgets::load_cards::MediaContents;
use std::sync::{Mutex, Arc};
use std::time::{UNIX_EPOCH, SystemTime};
use std::path::PathBuf;


#[derive(Clone)]
pub struct DepPair{
    dependent: u32,
    dependency: u32,

}



pub fn prev_id(conn: &Arc<Mutex<Connection>>) -> Result<u32>{
    Ok(conn.lock().unwrap().last_insert_rowid() as u32)

}


pub fn fetch_question(conn: &Arc<Mutex<Connection>>, cid: CardID) -> String {
    fetch_card(conn, cid).question
}


pub fn get_topic_of_card(conn: &Arc<Mutex<Connection>>, cid: CardID) -> TopicID {
    fetch_card(conn, cid).topic
}

pub fn fill_dependencies(conn: &Arc<Mutex<Connection>>, mut card: Card) -> Card {
    card.dependents = get_dependents(conn, card.id).unwrap();
    card.dependencies = get_dependencies(conn, card.id).unwrap();
    card
}

pub fn fetch_card(conn: &Arc<Mutex<Connection>>, cid: u32) -> Card {
    let card = conn.
        lock().
        unwrap().
        query_row(
        "SELECT * FROM cards WHERE id=?",
        [cid],
        |row| row2card(&row),
    ).expect(&format!("Failed to query following card: {}", cid));
    fill_dependencies(conn, card)
    
}



pub fn get_topics(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Topic>>{
    let mut vecoftops = Vec::<Topic>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM topics")?
        .query_map([], |row| {
            Ok(
              Topic{
                id: row.get(0)?,
                name: row.get(1)?,
                parent: row.get(2)?,
                children: Vec::<CardID>::new(),
                ancestors: 0,
                relpos: row.get(3)? 
            })
          }
        )?
        .for_each(
            |topic| {
                vecoftops.push(
                    topic.unwrap()
                    );

            }
            );
    Ok(vecoftops)
}



pub fn get_history(conn: &Arc<Mutex<Connection>>, id: u32) -> Result<Vec<Review>>{
    let mut vecofrows = Vec::<Review>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM revlog WHERE cid = ?")?
        .query_map([id], |row| {
            vecofrows.push(
            Review{
                grade: RecallGrade::from(row.get(2)?).unwrap(),
                date: row.get(0)?,
                answertime: row.get(3)?,
            }
            );
            Ok(())
        
    })?.for_each(|_|{});
    Ok(vecofrows)
}

// pub fn row2card(conn: &Arc<Mutex<Connection>>, row: &Row) -> Result<Card>{

pub fn row2card(row: &Row) -> Result<Card>{
    let cardtype = match row.get::<usize, u32>(7)?{
        0 => CardType::Pending,
        1 => CardType::Unfinished,
        2 => CardType::Finished,
        _ => panic!(),
    };
    let id = row.get(0)?;


    let frontaudio: Option<String> = row.get(3)?;
    let backaudio:  Option<String> = row.get(4)?; 
    let frontimage: Option<String> = row.get(5)?;
    let backimage:  Option<String> = row.get(6)?; 
    
    let frontaudio: Option<PathBuf> = frontaudio.map (|x| PathBuf::from(x));
    let backaudio:  Option<PathBuf> = backaudio.map  (|x| PathBuf::from(x));
    let frontimage: Option<PathBuf> = frontimage.map (|x| PathBuf::from(x));
    let backimage:  Option<PathBuf> = backimage.map  (|x| PathBuf::from(x));




  //  let dependencies = get_dependencies(conn, id).unwrap();
  //  let dependents = get_depndents(conn, id).unwrap();
        Ok(Card {
            id,
            question:      row.get(1)?,
            answer:        row.get(2)?,
            frontaudio,
            backaudio,
            frontimage,
            backimage,
            cardtype,
            suspended:     row.get(8)?,
            resolved:      row.get(9)?,
            dependents:  Vec::new(),
            dependencies: Vec::new(),
            topic:         row.get(10)?,
            source:        row.get(11)?,
        })
}


pub fn load_cards(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Card>> {
    let mut cardvec = Vec::<Card>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM cards")?
        .query_map([], |row| {
            cardvec.push(row2card(row).unwrap());
            Ok(())
        })?.for_each(|_|{});
    for i in 0..cardvec.len(){
        let id = cardvec[i].id;
        cardvec[i].dependencies = get_dependencies(conn, id).unwrap();
        cardvec[i].dependents = get_dependents(conn, id).unwrap();
    }
    
    Ok(cardvec)
}

pub fn load_card_matches(conn: &Arc<Mutex<Connection>>, search: &str) -> Result<Vec<Card>> {
    let mut cardvec = Vec::<Card>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM cards WHERE (question LIKE '%' || ?1 || '%') OR (answer LIKE '%' || ?1 || '%') LIMIT 50")?
        .query_map([search], |row| {
            cardvec.push(row2card(row).unwrap());
            Ok(())
        })?.for_each(|_|{});
    for i in 0..cardvec.len(){
        let id = cardvec[i].id;
        cardvec[i].dependencies = get_dependencies(conn, id).unwrap();
        cardvec[i].dependents   = get_dependents  (conn, id).unwrap();
    }
    Ok(cardvec)
}




pub fn get_dependents(conn: &Arc<Mutex<Connection>>, dependency: u32) -> Result<Vec<u32>>{
    let mut depvec = Vec::<CardID>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT dependent FROM dependencies where dependency = ?")?
        .query_map([dependency], |row| {
                depvec.push(row.get(0).unwrap());
                Ok(())
        
    })?.for_each(|_|{});
    Ok(depvec)
}

pub fn get_dependencies(conn: &Arc<Mutex<Connection>>, dependent: CardID) -> Result<Vec<CardID>>{
    let mut depvec = Vec::<CardID>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT dependency FROM dependencies where dependent = ?")?
        .query_map([dependent], |row| {
                depvec.push(row.get(0).unwrap());
                Ok(())
        
    })?.for_each(|_|{});
    Ok(depvec)
}

use crate::utils::card::CardType;



use crate::utils::incread::IncRead;
use crate::utils::widgets::textinput::Field;
use crate::utils::statelist::StatefulList;

struct IncTemp{
    parent: u32,
    topic: u32,
    source: String,
    isactive: bool,
}


pub fn fetch_media(conn: &Arc<Mutex<Connection>>, id: CardID) -> MediaContents{
    let card = fetch_card(conn, id);
    MediaContents{
        frontaudio: card.frontaudio,
        backaudio:  card.backaudio,
        frontimage: card.frontimage,
        backimage:  card.backimage,
    }
}

/* 


    conn.lock().unwrap().query_row(
    "select skipduration FROM unfinished_cards WHERE id=?",
    [id],
    |row| row.get(0),
    )



   */
pub fn get_incread(conn: &Arc<Mutex<Connection>>, id: u32) -> Result<IncRead>{
    let extracts    = load_extracts(conn, id).unwrap();
    let cloze_cards = load_cloze_cards(conn, id).unwrap();
    conn
        .lock()
        .unwrap()
        .query_row(
            "SELECT * FROM incread WHERE id = ?", 
            [id], 
            |row|{
                Ok(
                    IncRead{
                        id,
                        parent: row.get(1)?,
                        topic: row.get(2)?,
                        source: Field::new_with_text(row.get(3).unwrap(), row.get(7).unwrap(), row.get(8).unwrap()),
                        extracts: StatefulList::with_items(extracts),
                        clozes: StatefulList::with_items(cloze_cards),
                        isactive: row.get(4)?,
                    }
                )
            }
        )
}


use crate::utils::incread::IncListItem;

pub fn load_inc_items(conn: &Arc<Mutex<Connection>>, topic: TopicID) -> Result<Vec<IncListItem>> {
    let mut incvec = Vec::<IncListItem>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM incread where parent = 0 and topic = ?")
        .unwrap()
        .query_map(
            [topic],
            |row| {
                incvec.push(
                    IncListItem {
                        text: row.get(3)?,
                        id: row.get(0)?,
                    }
                    );
                Ok(())
            }
        )?.for_each(|_|{});
    Ok(incvec)
}

pub fn load_extracts(conn: &Arc<Mutex<Connection>>, parent: IncID) -> Result<Vec<IncListItem>> {
    let mut incvec = Vec::<IncListItem>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM incread where parent = ?")
        .unwrap()
        .query_map(
            [parent],
            |row| {
                incvec.push(
                    IncListItem {
                        text: row.get(3)?,
                        id: row.get(0)?,
                    }
                    );
                Ok(())
            }
        )?.for_each(|_|{});
    Ok(incvec)
}


/*


let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32;
if current_time - card.skiptime(conn) > card.skipduration(conn) * 84_600{
    unfinished_cards.push(card.id);
}

   
   */
pub fn load_active_inc(conn: &Arc<Mutex<Connection>>) -> Result<Vec<IncID>>{
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32;
    let mut incvec = Vec::<IncID>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT id FROM incread where active = 1 and ((? - skiptime) > (skipduration * 86400))")
        .unwrap()
        .query_map(
            [current_time], 
            |row| {
                incvec.push(row.get(0).unwrap());
                Ok(())
            }
        )?.for_each(|_|{});
    Ok(incvec)
}

use crate::utils::widgets::cardlist::CardItem;
pub fn load_inc_text(conn: &Arc<Mutex<Connection>>, id: IncID) -> Result<String>{
    conn
        .lock()
        .unwrap()
        .prepare("SELECT source FROM incread where id = ?")
        .unwrap()
        .query_row(
            [id], 
            |row| {
                Ok(row.get(0).unwrap())
            }
        )
}


pub fn load_inc_title(conn: &Arc<Mutex<Connection>>, incid: IncID, titlelen: u16) -> Result<String> {
    let mut source = load_inc_text(conn, incid).unwrap();
    source.truncate(titlelen.into());
    if source.len() < 5 {
        source = "Empty Source".to_string();
    }
    Ok(source)
}

pub fn get_topic_of_inc(conn: &Arc<Mutex<Connection>>, id: IncID) -> Result<TopicID>{
    conn
        .lock()
        .unwrap()
        .prepare("SELECT topic FROM incread where id = ?")
        .unwrap()
        .query_row(
            [id], 
            |row| {
                Ok(row.get(0).unwrap())
            }
        )
}
pub fn load_cloze_cards(conn: &Arc<Mutex<Connection>>, source: IncID) -> Result<Vec<CardItem>>{
    let mut clozevec = Vec::<CardItem>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM cards where source = ?")
        .unwrap()
        .query_map(
            [source], 
            |row| {
                clozevec.push(
                    CardItem{
                        question: row.get(1).unwrap(),
                        id: row.get(0).unwrap(),
                    }
                    );
                Ok(())
            }
        )?.for_each(|_|{});
    Ok(clozevec)
}



pub fn get_skipduration(conn: &Arc<Mutex<Connection>>, id: CardID) -> Result<u32> {
    conn.lock().unwrap().query_row(
    "select skipduration FROM unfinished_cards WHERE id=?",
    [id],
    |row| row.get(0),
    )
}

pub fn get_inc_skipduration(conn: &Arc<Mutex<Connection>>, id: IncID) -> Result<u32> {
    conn.lock().unwrap().query_row(
    "select skipduration FROM incread WHERE id=?",
    [id],
    |row| row.get(0),
    )
}


pub fn get_skiptime(conn: &Arc<Mutex<Connection>>, id: CardID) -> Result<u32> {
    conn.lock().unwrap().query_row(
    "select skiptime FROM unfinished_cards WHERE id=?",
    [id],
    |row| row.get(0),
    )
}


pub fn get_stability(conn: &Arc<Mutex<Connection>>, id: CardID) -> f32 {
    conn.lock().unwrap().query_row(
    "select stability FROM finished_cards WHERE id=?",
    [id],
    |row| row.get(0),
    ).expect(&format!("Couldn't find stability of card with following id: {}", id))
}


pub fn get_strength(conn: &Arc<Mutex<Connection>>, id: CardID) -> Result<f32> {
    conn.lock().unwrap().query_row(
    "select strength FROM finished_cards WHERE id=?",
    [id],
    |row| row.get(0),
    )
}


pub fn is_resolved(conn: &Arc<Mutex<Connection>>, id: CardID) -> bool{
    conn.lock().unwrap().query_row(
    "select resolved FROM cards WHERE id=?",
    [id],
    |row| row.get(0),
    ).unwrap()
}




//use crate::utils::card::CardType;
pub fn get_cardtype(conn: &Arc<Mutex<Connection>>, id: CardID) -> CardType{
    match conn.lock().unwrap().query_row(
    "select cardtype FROM cards WHERE id=?",
    [id],
    |row| row.get::<usize, usize>(0 as usize),
    ).unwrap() {
        0 => CardType::Pending,
        1 => CardType::Unfinished,
        2 => CardType::Finished,
        _ => panic!(),
    }
}




















