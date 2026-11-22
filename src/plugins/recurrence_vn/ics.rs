//! RFC 5545 iCalendar serializer with CRLF and `<= 75`-octet UTF-8 line folding.

use chrono::NaiveDate;

use crate::contracts::recurrence::{RecurrenceRequest, RecurrenceResponse};

fn escape_ics_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

/// Folds a single logical RFC 5545 content line so every emitted line is `<= 75` UTF-8 octets.
fn push_folded_line(out: &mut String, line: &str) {
    if line.len() <= 75 {
        out.push_str(line);
        out.push_str("\r\n");
        return;
    }

    let mut start = 0usize;
    let mut is_first = true;
    while start < line.len() {
        let max_chunk = if is_first { 75 } else { 74 };
        let rest = &line[start..];
        let mut cut = 0usize;
        for (idx, ch) in rest.char_indices() {
            let next_end = idx + ch.len_utf8();
            if next_end > max_chunk {
                break;
            }
            cut = next_end;
        }
        if cut == 0 {
            if let Some(ch) = rest.chars().next() {
                cut = ch.len_utf8();
            } else {
                break;
            }
        }
        if !is_first {
            out.push(' ');
        }
        out.push_str(&rest[..cut]);
        out.push_str("\r\n");
        start += cut;
        is_first = false;
    }
}

/// Renders an expanded [`RecurrenceResponse`] into an RFC 5545 VCALENDAR payload.
pub(crate) fn render_ics(req: &RecurrenceRequest, resp: &RecurrenceResponse) -> String {
    let mut out = String::with_capacity(resp.occurrences.len().saturating_mul(256) + 256);
    push_folded_line(&mut out, "BEGIN:VCALENDAR");
    push_folded_line(&mut out, "VERSION:2.0");
    push_folded_line(
        &mut out,
        "PRODID:-//VietCalendar//Lunar Recurrence Engine 0.2//VI",
    );
    push_folded_line(&mut out, "CALSCALE:GREGORIAN");
    push_folded_line(&mut out, "METHOD:PUBLISH");

    let base_desc = req
        .description
        .as_deref()
        .unwrap_or("Vietnamese Lunar Calendar recurring observance (UTC+7)");
    let summary_base = escape_ics_text(&resp.title);
    let desc_escaped = escape_ics_text(base_desc);

    for occ in &resp.occurrences {
        let Ok(solar) = NaiveDate::parse_from_str(&occ.solar_date, "%Y-%m-%d") else {
            continue;
        };
        let dt_start = solar.format("%Y%m%d").to_string();
        let dt_end = solar
            .succ_opt()
            .unwrap_or(solar)
            .format("%Y%m%d")
            .to_string();
        let leap_tag = if occ.is_leap_month_occurrence {
            "L"
        } else {
            "R"
        };
        let uid = format!(
            "vn-lunar-{}-{:04}{:02}{:02}-{}@vietcalendar.local",
            dt_start, occ.lunar.year, occ.lunar.month, occ.lunar.day, leap_tag
        );
        let summary_line = format!(
            "SUMMARY:{} ({:02}/{:02}{}{})",
            summary_base,
            occ.lunar.day,
            occ.lunar.month,
            if occ.lunar.is_leap { " nhuận" } else { "" },
            if occ.adjusted_from_day30 {
                " [29-day adj]"
            } else {
                ""
            }
        );
        let full_desc = format!(
            "DESCRIPTION:{}\\nLunar: {:02}/{:02}/{}{}",
            desc_escaped,
            occ.lunar.day,
            occ.lunar.month,
            occ.lunar.year,
            if occ.lunar.is_leap {
                " (Leap month)"
            } else {
                ""
            }
        );

        push_folded_line(&mut out, "BEGIN:VEVENT");
        push_folded_line(&mut out, &format!("UID:{uid}"));
        push_folded_line(&mut out, &format!("DTSTAMP:{dt_start}T000000Z"));
        push_folded_line(&mut out, &format!("DTSTART;VALUE=DATE:{dt_start}"));
        push_folded_line(&mut out, &format!("DTEND;VALUE=DATE:{dt_end}"));
        push_folded_line(&mut out, &summary_line);
        push_folded_line(&mut out, &full_desc);

        if let Some(mins) = req.reminder_minutes {
            push_folded_line(&mut out, "BEGIN:VALARM");
            push_folded_line(&mut out, "ACTION:DISPLAY");
            push_folded_line(&mut out, &format!("TRIGGER:-PT{mins}M"));
            push_folded_line(&mut out, &format!("DESCRIPTION:Reminder: {summary_base}"));
            push_folded_line(&mut out, "END:VALARM");
        }

        push_folded_line(&mut out, "END:VEVENT");
    }

    push_folded_line(&mut out, "END:VCALENDAR");
    out
}
