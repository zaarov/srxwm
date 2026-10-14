use x11rb::protocol::xproto::Rectangle;

use crate::x11::DockStrut;

pub(super) fn calculate(screen: Rectangle, docks: impl Iterator<Item = DockStrut>) -> Rectangle {
    let mut left = 0;
    let mut right = 0;
    let mut top = 0;
    let mut bottom = 0;

    for strut in docks {
        left = left.max(strut.left());
        right = right.max(strut.right());
        top = top.max(strut.top());
        bottom = bottom.max(strut.bottom());
    }

    let screen_width = u32::from(screen.width);
    let screen_height = u32::from(screen.height);
    left = left.min(screen_width);
    right = right.min(screen_width - left);
    top = top.min(screen_height);
    bottom = bottom.min(screen_height - top);

    Rectangle {
        x: clamp_i16(i32::from(screen.x) + left as i32),
        y: clamp_i16(i32::from(screen.y) + top as i32),
        width: (screen_width - left - right) as u16,
        height: (screen_height - top - bottom) as u16,
    }
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}
