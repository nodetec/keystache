use std::time::{Duration, Instant};

use crate::app;
use crate::util::lighten;
use iced::advanced::layout::{self, Layout, Limits};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::event::{self, Event};
use iced::widget::{column, container, horizontal_space, row, text};
use iced::Border;
use iced::{mouse, Color, Font};
use iced::{window, Shadow};
use iced::{Alignment, Element, Length, Rectangle, Renderer, Size, Theme, Vector};

use super::{mini_icon_button_no_text, PaletteColor, SvgIcon};

const DEFAULT_TIMEOUT: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastStatus {
    Neutral,
    Good,
    Bad,
}

impl ToastStatus {
    fn get_style(self, theme: &Theme) -> container::Style {
        let gray = lighten(theme.palette().background, 0.1);

        let border_color = match self {
            Self::Neutral => gray,
            Self::Good => theme.palette().success,
            Self::Bad => theme.palette().danger,
        };

        container::Style {
            background: Some(gray.into()),
            text_color: Color::WHITE.into(),
            border: Border {
                color: border_color,
                width: 1.,
                radius: (4.).into(),
            },
            shadow: Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.25),
                offset: Vector::new(-2., -2.),
                blur_radius: 4.,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub title: String,
    pub body: String,
    pub status: ToastStatus,
}

pub struct ToastManager<'a> {
    toasts: Vec<Element<'a, app::Message>>,
    timeout_secs: u64,
    on_close: Box<dyn Fn(usize) -> app::Message + 'a>,
}

impl<'a> ToastManager<'a> {
    pub fn new(toasts: &'a [Toast], on_close: impl Fn(usize) -> app::Message + 'a) -> Self {
        let toasts = toasts
            .iter()
            .enumerate()
            .map(|(index, toast)| {
                let close_button =
                    mini_icon_button_no_text(SvgIcon::Close, PaletteColor::Background);

                container(column![container(column![
                    row![
                        text(toast.title.as_str()).font(Font {
                            family: iced::font::Family::default(),
                            weight: iced::font::Weight::Bold,
                            stretch: iced::font::Stretch::Normal,
                            style: iced::font::Style::Normal,
                        }),
                        horizontal_space(),
                        close_button.on_press((on_close)(index))
                    ]
                    .align_y(Alignment::Center),
                    text(toast.body.as_str())
                ])
                .width(Length::Fill)
                .padding(16)
                .style(|theme| toast.status.get_style(theme))])
                .max_width(256)
                .into()
            })
            .collect();

        Self {
            toasts,
            timeout_secs: DEFAULT_TIMEOUT,
            on_close: Box::new(on_close),
        }
    }
}

impl<'a> Widget<app::Message, Theme, Renderer> for ToastManager<'a> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> layout::Node {
        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            limits,
            Length::Fill,
            Length::Fill,
            10.into(),
            10.0,
            Alignment::End,
            &self.toasts,
            &mut tree.children,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((child, state), layout) in self
            .toasts
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
        {
            child
                .as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        struct Marker;
        widget::tree::Tag::of::<Marker>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Vec::<Option<Instant>>::new())
    }

    fn children(&self) -> Vec<Tree> {
        self.toasts.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let instants = tree.state.downcast_mut::<Vec<Option<Instant>>>();

        // Invalidating removed instants to None allows us to remove
        // them here so that diffing for removed / new toast instants
        // is accurate
        instants.retain(Option::is_some);

        match (instants.len(), self.toasts.len()) {
            (old, new) if old > new => {
                instants.truncate(new);
            }
            (old, new) if old < new => {
                instants.extend(std::iter::repeat(Some(Instant::now())).take(new - old));
            }
            _ => {}
        }

        tree.diff_children(&self.toasts.iter().collect::<Vec<_>>());
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.toasts
                .iter()
                .zip(state.children.iter_mut())
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, app::Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let instants = state.state.downcast_mut::<Vec<Option<Instant>>>();

        if let Event::Window(window::Event::RedrawRequested(now)) = &event {
            let mut next_redraw: Option<window::RedrawRequest> = None;

            instants
                .iter_mut()
                .enumerate()
                .for_each(|(index, maybe_instant)| {
                    if let Some(instant) = maybe_instant.as_mut() {
                        let remaining = Duration::from_secs(self.timeout_secs)
                            .saturating_sub(instant.elapsed());

                        if remaining == Duration::ZERO {
                            maybe_instant.take();
                            shell.publish((self.on_close)(index));
                            next_redraw = Some(window::RedrawRequest::NextFrame);
                        } else {
                            let redraw_at = window::RedrawRequest::At(*now + remaining);
                            next_redraw = next_redraw
                                .map(|redraw| redraw.min(redraw_at))
                                .or(Some(redraw_at));
                        }
                    }
                });

            if let Some(redraw) = next_redraw {
                shell.request_redraw(redraw);
            }
        }

        let viewport = layout.bounds();

        self.toasts
            .iter_mut()
            .zip(state.children.iter_mut())
            .zip(layout.children())
            .zip(instants.iter_mut())
            .map(|(((child, state), layout), instant)| {
                let mut local_messages = vec![];
                let mut local_shell = Shell::new(&mut local_messages);

                let status = child.as_widget_mut().on_event(
                    state,
                    event.clone(),
                    layout,
                    cursor,
                    renderer,
                    clipboard,
                    &mut local_shell,
                    &viewport,
                );

                if !local_shell.is_empty() {
                    instant.take();
                }

                shell.merge(local_shell, std::convert::identity);

                status
            })
            .fold(event::Status::Ignored, event::Status::merge)
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.toasts
            .iter()
            .zip(state.children.iter())
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }
}

impl<'a> From<ToastManager<'a>> for Element<'a, app::Message> {
    fn from(manager: ToastManager<'a>) -> Self {
        Element::new(manager)
    }
}
