//! The mouse edge queue: short clicks survive until the game polls them.

use dos::MouseInput;

#[test]
fn a_quick_click_is_seen_as_a_press_then_a_release() {
    let mut mouse = MouseInput::default();
    mouse.move_to(123, 87);
    mouse.buttons(1);
    mouse.buttons(0);
    mouse.move_to(300, 190);
    mouse.poll();
    assert_eq!((mouse.sample().x, mouse.sample().y, mouse.sample().buttons), (123, 87, 1));
    mouse.poll();
    assert_eq!(
        (mouse.sample().x, mouse.sample().buttons),
        (123, 0),
        "the release keeps the click position"
    );
    mouse.poll();
    assert_eq!((mouse.sample().x, mouse.sample().y), (300, 190), "then the current position");
}

#[test]
fn clicks_older_than_a_second_of_emulated_time_expire() {
    let mut mouse = MouseInput::default();
    mouse.set_time(1000);
    mouse.buttons(1);
    mouse.buttons(0);
    mouse.set_time(1999);
    mouse.poll();
    assert_eq!(mouse.sample().buttons, 1, "999 ms is young enough");
    mouse.clear();
    mouse.set_time(1000);
    mouse.buttons(1);
    mouse.buttons(0);
    mouse.set_time(3000);
    mouse.poll();
    assert_eq!(mouse.sample().buttons, 0, "a click made during loading is dropped");
}

#[test]
fn positions_are_clamped_to_the_screen() {
    let mut mouse = MouseInput::default();
    mouse.move_to(-5, 500);
    assert_eq!((mouse.current().x, mouse.current().y), (0, 199));
}
