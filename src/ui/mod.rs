
use node::nethandle::NetHandle;
use node::Message;
use node::FromNetMsg;

use cursive::*;
use cursive::align::VAlign::Bottom;
use cursive::theme::PaletteColor::*;
use cursive::theme::Color::*;
use cursive::theme::BorderStyle;
use cursive::traits::*;
use cursive::event::Key;
use cursive::Cursive;
use std::thread;
use chrono::offset::Utc;
use chrono::DateTime;
use cursive::view::*;
use cursive::views::*;
use std::sync::{Arc, Mutex};

static mut HISTORY: Option<Arc<Mutex<Vec<Message>>>> = None;

pub fn cursive_main(neth: Arc<Mutex<NetHandle>>) {
    let neth_clone1 = neth.clone();
    let neth_clone2 = neth.clone();

    unsafe {
        // initialize the global variable
        HISTORY = Some(Arc::new(Mutex::new(Vec::new())));
    }

    let mut cursive = Cursive::ncurses();
    let sender = cursive.cb_sink().clone();

    cursive.set_fps(10);
    let _jh = thread::spawn(move || {
        loop {
            let opt = neth_clone2.lock().unwrap().read().expect("nethandle is dead");

            match opt {
                Some(FromNetMsg::NewMsg(msg)) => {
                    let mut hist = unsafe {
                        HISTORY.as_ref().unwrap().lock().unwrap()
                    };
                    hist.push(msg.clone());
                    sender.send(Box::new(move |s: &mut Cursive| {
                        let mut output = s.find_id::<TextView>("output").unwrap();
                        output.append(format_message(&msg).as_str());
                        output.append("\n");
                    })).unwrap();
                }
                Some(FromNetMsg::NotSent) => {
                    sender.send(Box::new(move |s: &mut Cursive| {
                        let mut output = s.find_id::<TextView>("output").unwrap();
                        output.append("The previous message was not sent\n");
                    })).unwrap();
                }
                _ => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
            }

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
                            Panel::new(ScrollView::new(
                                TextView::new("")
                                    .v_align(Bottom)
                                    .with_id("output"))
                                .scroll_strategy(ScrollStrategy::StickToBottom))))
        .child(BoxView::new(SizeConstraint::Full,
                            SizeConstraint::Fixed(5),
                            Panel::new(OnEventView::new(TextArea::new()
                                                     .content("")
                                                     .with_id("input"))
                                          .on_pre_event(Key::Enter, move |c| {
                                              let mut input = c.find_id::<TextArea>("input").unwrap();
                                              let neth = neth_clone1.lock().unwrap();
                                              neth.send_message(String::from(input.get_content())).unwrap();

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
