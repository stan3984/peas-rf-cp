
use node::nethandle::NetHandle;
use node::Message;
use node::FromNetMsg;

use pancurses::*;
use cursive::*;
use cursive::align::VAlign::Bottom;
use cursive::theme::PaletteColor::*;
use cursive::theme::Color::*;
use cursive::theme::BaseColor::*;
use cursive::theme::BorderStyle;
use cursive::theme::Theme;
use cursive::theme::ColorStyle;
use cursive::traits::*;
use cursive::event::{Event, Key};
use cursive::vec::Vec2;
use cursive::{Cursive, Printer};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use chrono::offset::Utc;
use chrono::DateTime;
use std::net::SocketAddr;
use cursive::view::*;
use cursive::views::*;
use common::id::Id;
use std::cell::Cell;
use std::sync::{Arc, Mutex};
use rand::Rng;



static mut history: Option<Arc<Mutex<Vec<Message>>>> = None;

pub fn cursive_main(neth: Arc<Mutex<NetHandle>>) {
    let neth_clone1 = neth.clone();
    let neth_clone2 = neth.clone();

    unsafe {
        // initialize the global variable
        history = Some(Arc::new(Mutex::new(Vec::new())));
    }

    let mut cursive = Cursive::ncurses();
    let sender = cursive.cb_sink().clone();

    cursive.set_fps(10);
    let jh = thread::spawn(move || {
        let mut done = false;
        while !done {
            let opt = neth_clone2.lock().unwrap().read().expect("nethandle is dead");

            match opt {
                Some(FromNetMsg::NewMsg(msg)) => {
                    let mut hist = unsafe {
                        history.as_ref().unwrap().lock().unwrap()
                    };
                    hist.push(msg);
                }
                _ => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                },
            }

            sender.send(Box::new(move |s: &mut Cursive| {
                let mut hist = unsafe {
                    history.as_ref().unwrap().lock().unwrap()
                };
                let mut output = s.find_id::<TextView>("output").unwrap();
                let newest = &hist[hist.len()-1];
                output.append(format_message(newest).as_str());
                output.append("\n");
            }));
        }
    });


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
                                              let text = input.get_content().to_string();

                                              // send message to others
                                              let mut neth = neth_clone1.lock().unwrap();
                                              neth.send_message(text.clone()).unwrap();

                                              // show the message to ourselves
                                              let mut output = c.find_id::<TextView>("output").unwrap();
                                              let mut append = format_message(&Message::new(text, Id::from_u64(0), String::from("<you>"), true));
                                              append.push('\n');
                                              output.append(append);

                                              input.set_content("");
                                          }
                                      )))));

    cursive.add_global_callback(event::Key::Esc, |c| c.quit());

    cursive.run();
}

fn format_message(msg: &Message) -> String {
    let s_name = msg.get_sender_name();
    let t_stamp = msg.get_timestamp();
    let datetime: DateTime<Utc> = t_stamp.into();
    format!("[{}]-[{}]: {}", datetime.format("%T"), s_name, msg.get_message())

}
