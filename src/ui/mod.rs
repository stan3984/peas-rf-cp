
use node::nethandle::NetHandle;
use node::Message;
use node::FromNetMsg;

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
                                              let mut neth = neth_clone1.lock().unwrap();
                                              neth.send_message(String::from(input.get_content())).unwrap();

                                              input.set_content("");
                                          }
                                      )))));

    cursive.add_global_callback(event::Key::Esc, |c| c.quit());

    cursive.run();
}

fn format_message(msg: &Message) -> String {
    "omg".to_string()
}
