
use node::nethandle::NetHandle;

use pancurses::*;
use cursive::*;
use cursive::align::VAlign::Bottom;
use cursive::theme::PaletteColor::*;
use cursive::theme::Color::*;
use cursive::theme::BaseColor::*;
use cursive::theme::BorderStyle;
use cursive::theme::Theme;
use cursive::traits::*;
use cursive::event::{Event, Key};
use cursive::vec::Vec2;
use cursive::{Cursive, Printer};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::net::SocketAddr;
use cursive::view::*;
use cursive::views::*;
use common::id::Id;
use std::cell::Cell;


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

    let history = Vec::new();

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
                                          .on_pre_event(Key::Enter, |c| {
                                              let mut input = c.find_id::<TextArea>("input").unwrap();
                                              let mut output = c.find_id::<TextView>("output").unwrap();
                                              history.push(neth.send_message(String::from(input.get_content())));
                                              input.set_content("");
                                          }
                                      )))));

    cursive.add_global_callback(event::Key::Esc, |c| c.quit());

    cursive.run();
}
