//! Dynamic single-month and 12-month calendar tooltip surface (`Size::auto()`, right-aligned).

use waywi::prelude::*;
use waywi::system::{CalendarMonth, TimeFormat, shift_year_month};

use crate::app::state::{Msg, State};
use crate::theme::{
    BLUE, CRUST, MAUVE, OVERLAY0, POPUP_TOP_MARGIN, RED, SURFACE1, TEXT, font_ui, font_ui_bold,
};

pub fn build_calendar_tooltip_surface(state: &State) -> Surface<Msg> {
    let (base_y, base_m) = state.calendar_target_ym();
    let count = if state.cal_year_mode { 12 } else { 1 };
    let cols_per_row = if state.cal_year_mode { 3 } else { 1 };
    let today = Some((state.now.year, state.now.month, state.now.day));

    let months: Vec<CalendarMonth> = (0..count)
        .map(|i| {
            let (y, m) = shift_year_month(base_y, base_m, i);
            CalendarMonth::build(y, m, today)
        })
        .collect();

    let mut grid_rows = Column::new().size(Size::auto()).gap(10.logical_px());
    for chunk in months.chunks(cols_per_row) {
        let mut r = Row::new().size(Size::auto()).gap(16.logical_px());
        for m in chunk {
            r = r.child(build_single_month_widget(m));
        }
        grid_rows = grid_rows.child(r);
    }

    let content = Column::new()
        .size(Size::auto())
        .padding(Edges::all(10.logical_px()))
        .gap(8.logical_px())
        .child(
            Text::new(state.now.format_hms(TimeFormat::H24))
                .font(font_ui_bold())
                .font_size(15.logical_px())
                .color(TEXT),
        )
        .child(grid_rows);

    let card = Group::new()
        .key("calendar-popup-card")
        .size(Size::auto())
        .on_pointer_down(|ev| {
            if ev.button == Some(MouseButton::Right) {
                Msg::ClockPointerDown(ev)
            } else {
                Msg::Noop
            }
        })
        .on_scroll(Msg::ClockScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(6.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(content);

    Surface::floating("bar-calendar-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::RIGHT)
        .margin(Edges::top_right(
            POPUP_TOP_MARGIN.logical_px(),
            8.logical_px(),
        ))
        .size(Size::auto())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseCalendar)
        .on_key(|ev| {
            if ev.key == Key::Escape {
                Some(Msg::CloseCalendar)
            } else {
                None
            }
        })
        .child(card)
}

fn calendar_cell(
    width: f32,
    text: String,
    font: Font,
    font_size: f32,
    color: Color,
    bg: Option<Color>,
) -> Element<Msg> {
    let mut cell = Group::new().size(Size::new(width.logical_px(), 16.logical_px()));
    if let Some(fill) = bg {
        cell = cell.child(
            Rect::new()
                .size(Size::fill())
                .fill(fill)
                .corner_radius(3.logical_px()),
        );
    }
    cell.child(
        Text::new(text)
            .font(font)
            .font_size(font_size.logical_px())
            .color(color)
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center),
    )
    .into()
}

fn build_single_month_widget(month: &CalendarMonth) -> Column<Msg> {
    let mut col = Column::new().size(Size::auto()).gap(2.logical_px()).child(
        Text::new(month.title.clone())
            .font(font_ui_bold())
            .font_size(11.logical_px())
            .color(MAUVE),
    );

    let mut hdr = Row::new().size(Size::auto()).gap(4.logical_px());
    for wd in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
        hdr = hdr.child(calendar_cell(
            22.0,
            String::from(wd),
            font_ui_bold(),
            10.0,
            BLUE,
            None,
        ));
    }
    hdr = hdr.child(calendar_cell(
        26.0,
        String::from("Wk"),
        font_ui(),
        9.0,
        OVERLAY0,
        None,
    ));
    col = col.child(hdr);

    for row in &month.rows {
        let mut r = Row::new()
            .size(Size::auto())
            .gap(4.logical_px())
            .align(Align::Center);
        for cell in &row.days {
            let label = cell.day.map_or_else(String::new, |d| d.to_string());
            let cell_el = if cell.is_today {
                calendar_cell(22.0, label, font_ui_bold(), 10.0, CRUST, Some(RED))
            } else {
                calendar_cell(22.0, label, font_ui(), 10.0, TEXT, None)
            };
            r = r.child(cell_el);
        }
        r = r.child(calendar_cell(
            26.0,
            format!("W{:02}", row.iso_week),
            font_ui(),
            9.0,
            OVERLAY0,
            None,
        ));
        col = col.child(r);
    }
    col
}
