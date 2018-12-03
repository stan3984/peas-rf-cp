
use pancurses::*;
pub fn ui_main() {
    let window = initscr();

    window.keypad(true); // Set keypad mode
    mousemask(ALL_MOUSE_EVENTS, std::ptr::null_mut()); // Listen to all mouse events

    window.printw("Welcome to peas-rf-cp, a distributed chat program made in Rust!\n\nRun peas --create newroom to create a new chat room.\nRun peas --join [ip:port] to join a already existing room.\n\nPress 'q' to quit.");
    window.refresh();

    loop {
        match window.getch() {
            Some(Input::KeyMouse) => {
                if let Ok(mouse_event) = getmouse() {
                    window.mvprintw(5, 0,
                                    &format!("Mouse at {},{}", mouse_event.x, mouse_event.y),
                    );
                };
            }
            Some(Input::Character(x)) if x == 'q' => break,
            _ => (),
        }
    }
    endwin();
}
