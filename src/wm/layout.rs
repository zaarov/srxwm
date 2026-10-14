use x11rb::protocol::xproto::{Rectangle, Window};

use super::{Client, Layout};

#[derive(Clone, Copy)]
struct Region {
    x: i64,
    y: i64,
    width: u64,
    height: u64,
}

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

pub(super) fn calculate(
    layout: Layout,
    clients: &[Client],
    area: Rectangle,
    gap: u32,
    border: u32,
    master_width_percent: u32,
) -> Vec<(Window, Rectangle)> {
    let tiled: Vec<Window> = clients
        .iter()
        .filter(|client| !client.is_floating() && client.is_mapped())
        .map(Client::window)
        .collect();

    if tiled.is_empty() || layout == Layout::Floating {
        return Vec::new();
    }

    let region = inset_by_gap(area, gap);
    let mut result = Vec::with_capacity(tiled.len());

    match layout {
        Layout::MasterStack => arrange_master_stack(
            &tiled,
            region,
            gap,
            border,
            master_width_percent,
            &mut result,
        ),
        Layout::Tree => arrange_tree(&tiled, region, gap, border, &mut result),
        Layout::Horizontal => {
            arrange_linear(&tiled, region, Axis::Horizontal, gap, border, &mut result)
        }
        Layout::Vertical => {
            arrange_linear(&tiled, region, Axis::Vertical, gap, border, &mut result)
        }
        Layout::Tabbed => {
            let geometry = client_rect(region, border);
            result.extend(tiled.iter().copied().map(|window| (window, geometry)));
        }
        Layout::Floating => unreachable!("floating layout returned above"),
    }

    result
}

fn inset_by_gap(area: Rectangle, gap: u32) -> Region {
    let width = u64::from(area.width);
    let height = u64::from(area.height);
    let inset = u64::from(gap).min(width / 2).min(height / 2);

    Region {
        x: i64::from(area.x) + inset as i64,
        y: i64::from(area.y) + inset as i64,
        width: width.saturating_sub(inset.saturating_mul(2)),
        height: height.saturating_sub(inset.saturating_mul(2)),
    }
}

fn arrange_linear(
    windows: &[Window],
    region: Region,
    axis: Axis,
    gap: u32,
    border: u32,
    output: &mut Vec<(Window, Rectangle)>,
) {
    let count = windows.len();
    if count == 0 {
        return;
    }

    let gap = u64::from(gap);
    let border = u64::from(border);
    let border_total = border.saturating_mul(2).saturating_mul(count as u64);

    match axis {
        Axis::Horizontal => {
            let gaps_total = gap.saturating_mul(count.saturating_sub(1) as u64);
            let content_total = region
                .width
                .saturating_sub(gaps_total)
                .saturating_sub(border_total);
            let (base, remainder) = divide_evenly(content_total, count);
            let mut x = region.x;

            for (index, &window) in windows.iter().enumerate() {
                let width = base + if (index as u64) < remainder { 1 } else { 0 };
                output.push((
                    window,
                    rect(x, region.y, width, region.height.saturating_sub(2 * border)),
                ));
                x += width as i64 + (2 * border + gap) as i64;
            }
        }
        Axis::Vertical => {
            let gaps_total = gap.saturating_mul(count.saturating_sub(1) as u64);
            let content_total = region
                .height
                .saturating_sub(gaps_total)
                .saturating_sub(border_total);
            let (base, remainder) = divide_evenly(content_total, count);
            let mut y = region.y;

            for (index, &window) in windows.iter().enumerate() {
                let height = base + if (index as u64) < remainder { 1 } else { 0 };
                output.push((
                    window,
                    rect(region.x, y, region.width.saturating_sub(2 * border), height),
                ));
                y += height as i64 + (2 * border + gap) as i64;
            }
        }
    }
}

fn arrange_master_stack(
    windows: &[Window],
    region: Region,
    gap: u32,
    border: u32,
    master_width_percent: u32,
    output: &mut Vec<(Window, Rectangle)>,
) {
    if windows.len() == 1 {
        output.push((windows[0], client_rect(region, border)));
        return;
    }

    let gap = u64::from(gap);
    let border = u64::from(border);
    let stack_count = windows.len() - 1;
    let content_width = region
        .width
        .saturating_sub(gap)
        .saturating_sub(border.saturating_mul(4));

    let percent = u64::from(master_width_percent.clamp(1, 99));
    let (master_width, stack_width) = if content_width >= 2 {
        let master = (content_width.saturating_mul(percent) / 100).clamp(1, content_width - 1);
        (master, content_width - master)
    } else {
        (content_width, 0)
    };

    let full_height = region.height.saturating_sub(border.saturating_mul(2));
    output.push((
        windows[0],
        rect(region.x, region.y, master_width, full_height),
    ));

    let stack_x = region.x + (master_width + 2 * border + gap) as i64;
    let stack_region = Region {
        x: stack_x,
        y: region.y,
        width: stack_width.saturating_add(2 * border),
        height: region.height,
    };
    arrange_linear(
        &windows[1..],
        stack_region,
        Axis::Vertical,
        gap as u32,
        border as u32,
        output,
    );

    debug_assert_eq!(output.len(), stack_count + 1);
}

fn arrange_tree(
    windows: &[Window],
    region: Region,
    gap: u32,
    border: u32,
    output: &mut Vec<(Window, Rectangle)>,
) {
    if windows.is_empty() {
        return;
    }
    arrange_tree_recursive(
        windows,
        region,
        u64::from(gap),
        u64::from(border),
        0,
        output,
    );
}

fn arrange_tree_recursive(
    windows: &[Window],
    region: Region,
    gap: u64,
    border: u64,
    depth: usize,
    output: &mut Vec<(Window, Rectangle)>,
) {
    if windows.len() == 1 {
        output.push((windows[0], client_rect(region, border as u32)));
        return;
    }

    let first_count = windows.len() / 2;
    let horizontal_split = if region.width == region.height {
        depth % 2 == 0
    } else {
        region.width >= region.height
    };

    if horizontal_split {
        let space = region.width.saturating_sub(gap);
        let first_width = space.saturating_mul(first_count as u64) / windows.len() as u64;
        let second_width = space.saturating_sub(first_width);
        arrange_tree_recursive(
            &windows[..first_count],
            Region {
                width: first_width,
                ..region
            },
            gap,
            border,
            depth + 1,
            output,
        );
        arrange_tree_recursive(
            &windows[first_count..],
            Region {
                x: region.x + first_width as i64 + gap as i64,
                width: second_width,
                ..region
            },
            gap,
            border,
            depth + 1,
            output,
        );
    } else {
        let space = region.height.saturating_sub(gap);
        let first_height = space.saturating_mul(first_count as u64) / windows.len() as u64;
        let second_height = space.saturating_sub(first_height);
        arrange_tree_recursive(
            &windows[..first_count],
            Region {
                height: first_height,
                ..region
            },
            gap,
            border,
            depth + 1,
            output,
        );
        arrange_tree_recursive(
            &windows[first_count..],
            Region {
                y: region.y + first_height as i64 + gap as i64,
                height: second_height,
                ..region
            },
            gap,
            border,
            depth + 1,
            output,
        );
    }
}

fn client_rect(region: Region, border: u32) -> Rectangle {
    let border = u64::from(border);
    rect(
        region.x,
        region.y,
        region.width.saturating_sub(2 * border),
        region.height.saturating_sub(2 * border),
    )
}

fn divide_evenly(total: u64, count: usize) -> (u64, u64) {
    if count == 0 {
        return (0, 0);
    }

    let count = count as u64;
    (total / count, total % count)
}

fn rect(x: i64, y: i64, width: u64, height: u64) -> Rectangle {
    Rectangle {
        x: x.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16,
        y: y.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16,
        width: width.clamp(1, u64::from(u16::MAX)) as u16,
        height: height.clamp(1, u64::from(u16::MAX)) as u16,
    }
}
