
use crate::utils::widgets::textinput::CursorPos;
use crate::{MyKey, Direction};

use crate::logic::review::{UnfCard, UnfSelection, CardReview};
use crate::utils::sql::update::{update_inc_text,  update_card_question, update_card_answer, double_skip_duration, set_suspended};
use crate::utils::sql::fetch::get_cardtype;
use crate::utils::widgets::find_card::{FindCardWidget, CardPurpose};
use crate::app::{App, PopUp};
use crate::utils::card::{RecallGrade, Card, CardType};
use crate::logic::review::{ReviewSelection, IncSelection, IncMode};
use crate::logic::review::ReviewMode;

use rusqlite::Connection;
use crate::utils::aliases::*;
use crate::utils::widgets::newchild::AddChildWidget;

use crate::utils::widgets::newchild::Purpose;



use std::sync::{Arc, Mutex};
enum Action {
    IncNext(String, TopicID, CursorPos),
    IncDone(String, TopicID, CursorPos),
    Review(String, String, CardID, char),
    SkipUnf(String, String, CardID),
    SkipRev(String, String, CardID),
    CompleteUnf(String, String, CardID),
    NewDependency(CardID),
    NewDependent(CardID),
    AddDependency(CardID),
    AddDependent(CardID),
    AddChild(IncID),
    PlayBackAudio(CardID),
    Quit,
    Refresh,
    None,
}

pub fn review_event(app: &mut App, key: MyKey) {


    let mut action = Action::None;

    match &mut app.review.mode{
        ReviewMode::Done => mode_done(key, &mut action),
        ReviewMode::Unfinished(unf) => mode_unfinished(&app.conn, unf, key, &mut action),
        ReviewMode::Pending(rev) | ReviewMode::Review(rev) => mode_review(&app.conn, rev, key, &mut action),
        ReviewMode::IncRead(inc) => mode_inc(&app.conn, inc, key, &mut action),
}


    match action{
        Action::IncNext(source, id, cursor) => {
            app.review.inc_next(&app.conn, &app.audio_handle, id);
            update_inc_text(&app.conn, source, id, &cursor).unwrap();
        },
        Action::IncDone(source, id, cursor) => {
            app.review.inc_done(id, &app.conn, &app.audio_handle);
            update_inc_text(&app.conn, source, id, &cursor).unwrap();
        },
        Action::Review(question, answer, id, char) => {
            let grade = match char{
                '1' => RecallGrade::None,
                '2' => RecallGrade::Failed,
                '3' => RecallGrade::Decent,
                '4' => RecallGrade::Easy,
                _ => panic!("illegal argument"),
            };
            if get_cardtype(&app.conn, id) == CardType::Pending{
                Card::activate_card(&app.conn, id);
            }
            app.review.new_review(&app.conn, id, grade, &app.audio_handle);
            update_card_question(&app.conn, id, question).unwrap();
            update_card_answer(&app.conn, id, answer).unwrap();
        },
        Action::SkipUnf(question, answer, id) => {
            app.review.random_mode(&app.conn, &app.audio_handle);
            update_card_question(&app.conn, id, question).unwrap();
            update_card_answer(&app.conn, id, answer).unwrap();
            double_skip_duration(&app.conn, id).unwrap();
        },
        Action::SkipRev(question, answer, id) => {
            app.review.random_mode(&app.conn, &app.audio_handle);
            update_card_question(&app.conn, id, question).unwrap();
            update_card_answer(&app.conn, id, answer).unwrap();
        },
        Action::CompleteUnf(question, answer, id) => {
            Card::complete_card(&app.conn, id);
            app.review.random_mode(&app.conn, &app.audio_handle);
            update_card_question(&app.conn, id, question).unwrap();
            update_card_answer(&app.conn, id, answer).unwrap();
        },
        Action::NewDependency(id) => {
            let prompt = String::from("Add new dependency");
            let purpose = CardPurpose::NewDependency(id);
            let cardfinder = FindCardWidget::new(&app.conn, prompt, purpose);
            app.popup = PopUp::CardSelecter(cardfinder);
        },
        Action::NewDependent(id) => {
            let prompt = String::from("Add new dependent");
            let purpose = CardPurpose::NewDependent(id);
            let cardfinder = FindCardWidget::new(&app.conn, prompt, purpose);
            app.popup = PopUp::CardSelecter(cardfinder);
        },
        Action::AddDependent(id) => {
            let addchild = AddChildWidget::new(&app.conn, Purpose::Dependency(id));
            app.popup = PopUp::AddChild(addchild);
        },
        Action::AddDependency(id) => {
            let addchild = AddChildWidget::new(&app.conn, Purpose::Dependent(id));
            app.popup = PopUp::AddChild(addchild);
        },
        Action::AddChild(id) => {
            let addchild = AddChildWidget::new(&app.conn, Purpose::Source(id));
            app.popup = PopUp::AddChild(addchild);
        }
        Action::PlayBackAudio(id) => {
            Card::play_backaudio(&app.conn, id, &app.audio_handle);
        }
        Action::Quit => {
            app.should_quit = true;
        },
        Action::Refresh => {
            app.review = crate::logic::review::ReviewList::new(&app.conn, &app.audio_handle);
        },
        Action::None => {},
    }
}


fn unf_nav(rev: &mut UnfCard, dir: &Direction){
    use UnfSelection::*;
    use Direction::*;
    match (&rev.selection, dir){
        (Question, Right) => rev.selection = Dependents,
        (Question, Down)  => rev.selection = Answer,
        
        (Answer, Right)   => rev.selection = Dependencies,
        (Answer, Up)      => rev.selection = Question,

        (Dependencies, Left) => rev.selection = Answer,
        (Dependencies, Up)   => rev.selection = Dependents,

        (Dependents, Left)   => rev.selection = Question,
        (Dependents, Down)   => rev.selection = Dependencies,

        _ => {},
    }
}
fn rev_nav(rev: &mut CardReview, dir: &Direction){
    use ReviewSelection::*;
    use Direction::*;
    match (&rev.selection, dir){
        (Question, Right) => rev.selection = Dependents,
        (Question, Down) if rev.reveal => rev.selection = Answer,
        (Question, Down) => rev.selection = RevealButton,
        
        (Answer, Right)   => rev.selection = Dependencies,
        (Answer, Up)      => rev.selection = Question,
        (Answer, Down) if rev.reveal => rev.selection = CardRater,

        (Dependencies, Left) if rev.reveal => rev.selection = Answer,
        (Dependencies, Left) => rev.selection = RevealButton,
        (Dependencies, Up)   => rev.selection = Dependents,

        (Dependents, Left)   => rev.selection = Question,
        (Dependents, Down)   => rev.selection = Dependencies,

        (RevealButton, Right)   => rev.selection = Dependencies,
        (RevealButton, Up)   => rev.selection = Question,


        (CardRater, Right) => rev.selection = Dependencies,
        (CardRater, Up)    => rev.selection = Answer,
        _ => {},
    }
}
fn inc_nav(inc: &mut IncMode, dir: &Direction){
    use IncSelection::*;
    use Direction::*;
    match (&inc.selection, dir){
        (Source, Right) => inc.selection = Extracts,

        (Clozes, Up)   => inc.selection = Extracts,
        (Clozes, Left) => inc.selection = Source,

        (Extracts, Left) => inc.selection = Source,
        (Extracts, Down) => inc.selection = Clozes,
        _ => {},
    }
}

fn mode_inc(conn: &Arc<Mutex<Connection>>, inc: &mut IncMode, key: MyKey, action: &mut Action) {
    use MyKey::*;
    use IncSelection::*;
    
    if let MyKey::Nav(dir) = &key{
        inc_nav(inc, dir);
        return;
    }
    match (&inc.selection, key) {
        (_, Alt('q')) => *action = Action::Quit,
        (_, Alt('d')) => *action = Action::IncDone(inc.source.source.return_text(), inc.id, inc.source.source.cursor.clone()), 
        (_, Alt('s')) => *action = Action::IncNext(inc.source.source.return_text(), inc.id, inc.source.source.cursor.clone()),
        (Source, Alt('a')) => *action = Action::AddChild(inc.id),
        (Source, key) =>  inc.source.keyhandler(conn, key),
        (_,_) => {},
    }
}

fn mode_review(conn: &Arc<Mutex<Connection>>, unf: &mut CardReview, key: MyKey, action: &mut Action) {
    use ReviewSelection::*;
    use MyKey::*;
        
    if let MyKey::Nav(dir) = &key{
        rev_nav(unf, dir);
        return;
    }
    match (&unf.selection, key){
        (_, Alt('q')) => *action = Action::Quit,
        (_, Alt('s')) => *action = Action::SkipRev(unf.question.return_text(), unf.answer.return_text(), unf.id),
        (_, Alt('t')) => *action = Action::NewDependent(unf.id),
        (_, Alt('y')) => *action = Action::NewDependency(unf.id),
        (_, Alt('T')) => *action = Action::AddDependent(unf.id),
        (_, Alt('Y')) => *action = Action::AddDependency(unf.id),
        (_, Alt('i')) => {
            set_suspended(conn, unf.id, true).unwrap();
    *action = Action::SkipRev(unf.question.return_text(), unf.answer.return_text(), unf.id);
        },
        (RevealButton, Char(' ')) | (RevealButton, Enter) => {
            unf.reveal = true;
            unf.selection = CardRater;
            *action = Action::PlayBackAudio(unf.id);
        },
        (Question, key) => unf.question.keyhandler(key),
        (Answer,   key) => unf.answer.keyhandler(key),

        (CardRater, Char(num)) if num.is_digit(10) 
            && (1..5).contains(&num.to_digit(10).unwrap()) =>  {
                *action = Action::Review(
                    unf.question.return_text(), 
                    unf.answer.return_text(), 
                    unf.id, 
                    num,
            )},
        (CardRater, Char(' ')) | (CardRater, Enter) if unf.cardrater.selection.is_some()=> {
            let foo = unf.cardrater.selection.clone().unwrap();
            let num =  match foo{
                RecallGrade::None   => '1',
                RecallGrade::Failed => '2',
                RecallGrade::Decent => '3',
                RecallGrade::Easy   => '4',
            };
            *action = Action::Review(
                    unf.question.return_text(), 
                    unf.answer.return_text(), 
                    unf.id, 
                    num,
            )
        },
        (CardRater, Char(' ')) | (CardRater, Enter) => *action = Action::SkipRev(unf.question.return_text(), unf.answer.return_text(), unf.id),
        (CardRater, key) => unf.cardrater.keyhandler(key),
        (_,_) => {},
    }
}

fn mode_done(key: MyKey, action: &mut Action){
    match key{
        MyKey::Alt('q') => *action = Action::Quit,
        MyKey::Alt('r') => *action = Action::Refresh,

        _ => {},
    }
}

fn mode_unfinished(conn: &Arc<Mutex<Connection>>, unf: &mut UnfCard, key: MyKey, action: &mut Action) {
    use UnfSelection::*;
    use MyKey::*;

    if let MyKey::Nav(dir) = &key{
        unf_nav(unf, dir);
        return;
    }
    match (&unf.selection, key){
        (_, Alt('q')) => *action = Action::Quit,
        (_, Alt('s'))     => *action = Action::SkipUnf    (unf.question.return_text(), unf.answer.return_text(), unf.id),
        (_, Alt('f')) => *action = Action::CompleteUnf(unf.question.return_text(), unf.answer.return_text(), unf.id), 
        (_, Alt('t'))  => *action = Action::NewDependent(unf.id),
        (_, Alt('y'))  => *action = Action::NewDependency(unf.id),
        (_, Alt('T'))  => *action = Action::AddDependent(unf.id),
        (_, Alt('Y'))  => *action = Action::AddDependency(unf.id),
        (_, Alt('i')) => {
            set_suspended(conn, unf.id, true).unwrap();
            *action = Action::SkipRev(unf.question.return_text(), unf.answer.return_text(), unf.id);
        },
        (Question, key) => unf.question.keyhandler(key),
        (Answer,   key) => unf.answer.keyhandler(key),
        (_,_) => {},
    }
}
