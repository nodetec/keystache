use fedimint_core::Amount;
use iced::Color;
use palette::{rgb::Rgb, FromColor, Hsl};

pub fn darken(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.lightness = if hsl.lightness - amount < 0.0 {
        0.0
    } else {
        hsl.lightness - amount
    };

    from_hsl(hsl)
}

pub fn lighten(color: Color, amount: f32) -> Color {
    let mut hsl = to_hsl(color);

    hsl.lightness = if hsl.lightness + amount > 1.0 {
        1.0
    } else {
        hsl.lightness + amount
    };

    from_hsl(hsl)
}

fn to_hsl(color: Color) -> Hsl {
    Hsl::from_color(Rgb::from(color))
}

fn from_hsl(hsl: Hsl) -> Color {
    Rgb::from_color(hsl).into()
}

pub fn format_amount(amount: Amount) -> String {
    let amount_sats = amount.msats / 1000;
    let sub_sat_msats = amount.msats % 1000;

    if amount_sats == 1 && sub_sat_msats == 0 {
        return "1 sat".to_string();
    }

    let comma_formatted_sats = amount_sats
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",");

    let msats_str = if sub_sat_msats == 0 {
        String::new()
    } else {
        let mut sub_sat_msats_str = format!(".{sub_sat_msats:03}");
        while sub_sat_msats_str.ends_with('0') {
            sub_sat_msats_str.pop();
        }
        sub_sat_msats_str
    };

    format!("{comma_formatted_sats}{msats_str} sats")
}

/// Adds ellipses to a string if it exceeds a certain length, ensuring the total length is at most
/// `max_len` characters. Can either place the ellipses at the end of the string or in the center.
#[must_use]
pub fn truncate_text(input: &str, max_len: usize, center: bool) -> String {
    const ELLIPSES: &str = "...";
    const ELLIPSES_LEN: usize = ELLIPSES.len();

    let chars = input.chars().collect::<Vec<_>>();

    if chars.len() <= max_len {
        return input.to_string();
    }

    if max_len <= ELLIPSES_LEN {
        return ELLIPSES.to_string();
    }

    if center {
        // The number of total characters from `input` to display.
        // Subtract 3 for the ellipsis.
        let chars_to_display = max_len - 3;

        let is_lobsided = chars_to_display % 2 != 0;

        let chars_in_front = if is_lobsided {
            (chars_to_display / 2) + 1
        } else {
            chars_to_display / 2
        };

        let chars_in_back = chars_to_display / 2;

        format!(
            "{}{ELLIPSES}{}",
            &chars[..chars_in_front].iter().collect::<String>(),
            &chars[(chars.len() - chars_in_back)..]
                .iter()
                .collect::<String>()
        )
    } else {
        format!(
            "{}{ELLIPSES}",
            &chars[..(max_len - ELLIPSES_LEN)].iter().collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_amount_sats() {
        // 0 sats is plural.
        assert_eq!(format_amount(Amount::from_sats(0)), "0 sats");

        // 1 sat is singular.
        assert_eq!(format_amount(Amount::from_sats(1)), "1 sat");

        // Digits are ordered correctly.
        assert_eq!(format_amount(Amount::from_sats(1234)), "1,234 sats");

        // Commas are placed correctly.
        assert_eq!(format_amount(Amount::from_sats(10)), "10 sats");
        assert_eq!(format_amount(Amount::from_sats(100)), "100 sats");
        assert_eq!(format_amount(Amount::from_sats(1_000)), "1,000 sats");
        assert_eq!(format_amount(Amount::from_sats(10_000)), "10,000 sats");
        assert_eq!(format_amount(Amount::from_sats(100_000)), "100,000 sats");
        assert_eq!(
            format_amount(Amount::from_sats(1_000_000)),
            "1,000,000 sats"
        );

        // Millisats are displayed as sub-sats, without extra zeros.
        assert_eq!(format_amount(Amount::from_msats(1)), "0.001 sats");
        assert_eq!(format_amount(Amount::from_msats(10)), "0.01 sats");
        assert_eq!(format_amount(Amount::from_msats(100)), "0.1 sats");

        // Millisats are displayed properly with sats
        assert_eq!(
            format_amount(Amount::from_msats(123456789)),
            "123,456.789 sats"
        );
    }

    #[test]
    fn test_truncate_text() {
        // Test short input (no truncation needed).
        assert_eq!(truncate_text("Hello", 10, false), "Hello");
        assert_eq!(truncate_text("Hello", 10, true), "Hello");

        // Test input exactly matching `max_len`.
        assert_eq!(truncate_text("Hello", 5, false), "Hello");
        assert_eq!(truncate_text("Hello", 5, true), "Hello");

        // Test long input.
        assert_eq!(truncate_text("Hello, world!", 8, false), "Hello...");
        assert_eq!(truncate_text("Hello, world!", 8, true), "Hel...d!");

        // Test Unicode string handling.
        assert_eq!(truncate_text("こんにちは世界", 6, false), "こんに...");
        assert_eq!(truncate_text("こんにちは世界", 6, true), "こん...界");

        // Test empty input.
        assert_eq!(truncate_text("", 5, false), "");
        assert_eq!(truncate_text("", 5, true), "");

        // Test edge cases with small `max_len` values.
        assert_eq!(truncate_text("Hello, world!", 0, false), "...");
        assert_eq!(truncate_text("Hello, world!", 0, true), "...");
        assert_eq!(truncate_text("Hello, world!", 1, false), "...");
        assert_eq!(truncate_text("Hello, world!", 1, true), "...");
        assert_eq!(truncate_text("Hello, world!", 2, false), "...");
        assert_eq!(truncate_text("Hello, world!", 2, true), "...");
        assert_eq!(truncate_text("Hello, world!", 3, false), "...");
        assert_eq!(truncate_text("Hello, world!", 3, true), "...");
        assert_eq!(truncate_text("Hello, world!", 4, false), "H...");
        assert_eq!(truncate_text("Hello, world!", 4, true), "H...");
        assert_eq!(truncate_text("Hello, world!", 5, false), "He...");
        assert_eq!(truncate_text("Hello, world!", 5, true), "H...!");
        assert_eq!(truncate_text("Hello, world!", 6, false), "Hel...");
        assert_eq!(truncate_text("Hello, world!", 6, true), "He...!");
        assert_eq!(truncate_text("Hello, world!", 7, false), "Hell...");
        assert_eq!(truncate_text("Hello, world!", 7, true), "He...d!");
    }
}
