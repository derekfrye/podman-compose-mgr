use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

pub fn styled_key(content: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        content,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}
