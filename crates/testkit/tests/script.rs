//! Scenario scripts: timed desktop input, one event per line.

use testkit::script::{Action, Event, parse};

#[test]
fn each_line_is_a_time_an_event_and_its_arguments() {
    let script = "# a comment\n\n18000 key 16384\n19500 mouse 208 85\n20000 down 79 4096\n";
    assert_eq!(
        parse(script),
        Ok(vec![
            Event { at_ms: 18_000, action: Action::Key(16_384) },
            Event { at_ms: 19_500, action: Action::Mouse { x: 208, y: 85 } },
            Event { at_ms: 20_000, action: Action::Down { scancode: 79, mods: 4096 } },
        ])
    );
}

#[test]
fn missing_arguments_are_zero() {
    let events = parse("1050 down 81\n1100 focuslost\n").expect("a valid script");
    assert_eq!(events[0].action, Action::Down { scancode: 81, mods: 0 });
    assert_eq!(events[1].action, Action::FocusLost);
}

#[test]
fn every_event_type_parses() {
    let script = "1 key 283\n2 down 4\n3 up 4\n4 repeat 4 1\n5 focuslost\n6 ascii 65\n\
                  7 mouse 1 2\n8 buttons 3\n9 screen 300\n10 capture 200\n";
    let actions: Vec<Action> =
        parse(script).expect("valid").into_iter().map(|e| e.action).collect();
    assert_eq!(
        actions,
        [
            Action::Key(283),
            Action::Down { scancode: 4, mods: 0 },
            Action::Up { scancode: 4, mods: 0 },
            Action::Repeat { scancode: 4, mods: 1 },
            Action::FocusLost,
            Action::Ascii(65),
            Action::Mouse { x: 1, y: 2 },
            Action::Buttons(3),
            Action::Screen(300),
            Action::Capture(200),
        ]
    );
}

#[test]
fn scripts_must_be_in_time_order() {
    let error = parse("200 capture 1\n100 capture 2\n").expect_err("out of order");
    assert_eq!(error.line, 2);
}

#[test]
fn unknown_events_and_scancodes_are_rejected() {
    assert_eq!(parse("1 jump 3\n").expect_err("unknown").line, 1);
    assert_eq!(parse("1 capture 1\n2 down 512\n").expect_err("no such scancode").line, 2);
    assert_eq!(parse("1 down 0\n").expect_err("no such scancode").line, 1);
}
