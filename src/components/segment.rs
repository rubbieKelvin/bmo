use gpui::{Div, FontWeight, ParentElement, Styled, div, prelude::FluentBuilder, px, rgb, white};

use crate::utils::format_time;

#[derive(Debug, Clone)]
pub enum Segment {
    Value(String),
    Separator,
}

impl Segment {
    pub fn div(time: u128, on_break: bool) -> Div {
        let formatted_time = format_time(time);
        let values = into_segments(formatted_time);
        return segments(values, on_break);
    }
}

fn into_segments(formatted_time: String) -> Vec<Segment> {
    let values: Vec<String> = formatted_time
        .split(":")
        .into_iter()
        .map(|segment| segment.to_string())
        .collect();
    let mut res: Vec<Segment> = vec![];

    for (index, value) in values.iter().enumerate() {
        res.push(Segment::Value(value.clone()));
        if index != values.len() - 1 {
            res.push(Segment::Separator);
        }
    }

    return res;
}

fn segments(mut values: Vec<Segment>, is_other_state: bool) -> Div {
    match values.get(0) {
        Some(Segment::Value(c)) if c == "00" => {
            values.remove(0);
            values.remove(0);
        }
        _ => {}
    }
    return div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .children(values.iter().map(|segment| {
            match segment {
                Segment::Value(v) => div()
                    .child(v.to_string())
                    .px_1()
                    .text_size(px(54.))
                    .text_center()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(white())
                    .when(is_other_state, |el| el.text_color(rgb(0xFFFF95))),
                Segment::Separator => div()
                    .child(":")
                    .px_1()
                    .text_center()
                    .text_size(px(50.))
                    .text_color(rgb(0x777777))
                    .when(is_other_state, |el| el.text_color(rgb(0xFFFF95))),
            }
        }));
}
