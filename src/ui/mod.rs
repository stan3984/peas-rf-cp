
use node::nethandle::NetHandle;
use node::Message;

#[allow(unused_imports)] use pancurses::*;
#[allow(unused_imports)] use cursive::*;
#[allow(unused_imports)] use cursive::align::VAlign::Bottom;
#[allow(unused_imports)] use cursive::theme::PaletteColor::*;
#[allow(unused_imports)] use cursive::theme::Color::*;
#[allow(unused_imports)] use cursive::theme::BaseColor::*;
#[allow(unused_imports)] use cursive::theme::BorderStyle;
#[allow(unused_imports)] use cursive::theme::Theme;
#[allow(unused_imports)] use cursive::traits::*;
#[allow(unused_imports)] use cursive::event::{Event, Key};
#[allow(unused_imports)] use cursive::vec::Vec2;
#[allow(unused_imports)] use cursive::{Cursive, Printer};
#[allow(unused_imports)] use std::collections::VecDeque;
#[allow(unused_imports)] use std::sync::mpsc;
#[allow(unused_imports)] use std::thread;
#[allow(unused_imports)] use std::time::Duration;
#[allow(unused_imports)] use std::time::SystemTime;
#[allow(unused_imports)] use std::net::SocketAddr;
#[allow(unused_imports)] use cursive::view::*;
#[allow(unused_imports)] use cursive::views::*;
#[allow(unused_imports)] use common::id::Id;
#[allow(unused_imports)] use std::cell::Cell;
#[allow(unused_imports)] use std::sync::Arc;
#[allow(unused_imports)] use std::sync::Mutex;


static history_mutex: Option<Arc<Mutex<Vec<Message>>>> = None;

pub fn cursive_test(neth: NetHandle) {

    let mut cursive = Cursive::ncurses();

    /*
    let prefix = |msg| {
        let ts = msg.timestamp;
        let sn = msg.sender_name;
        format!("[{} {}] {}", sn, ts, msg)
    };
    */

    cursive.set_fps(10);
    //cursive.cb_sink().send();

    let mut cur = cursive.current_theme().clone();
    cur.palette[Background] = Rgb(64,64,64);
    cur.shadow = false;
    cur.palette[Primary] = Rgb(0,0,0);
    cur.palette[Secondary] = Rgb(64,64,64);
    cur.borders = BorderStyle::Simple;
    cursive.set_theme(cur);

    cursive.add_layer(LinearLayout::vertical()
        .child(BoxView::new(SizeConstraint::Full,
                            SizeConstraint::Full,
                            Panel::new(TextView::new("")
                                .v_align(Bottom)
                                .with_id("output"))))
        .child(BoxView::new(SizeConstraint::Full,
                            SizeConstraint::Fixed(5),
                            Panel::new(OnEventView::new(TextArea::new()
                                                     .content("")
                                                     .with_id("input"))
                                          .on_pre_event(Key::Enter, move |c| {
                                              let mut input = c.find_id::<TextArea>("input").unwrap();
                                              let mut output = c.find_id::<TextView>("output").unwrap();
                                              history_mutex.unwrap().lock()
                                                           .unwrap()
                                                           .push(neth.send_message(String::from(input.get_content())).unwrap());
                                              input.set_content("");
                                          }
                                      )))));

    cursive.add_global_callback(event::Key::Esc, |c| c.quit());

    cursive.run();
}
