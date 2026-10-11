//! A widget that opens a popup below itself when clicked, like a drop-down
//! menu. `pick_list` would do, but its menu is only as wide as its button.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{overlay, renderer, Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::{mouse, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

/// Space between the anchor and the popup.
const GAP: f32 = 4.0;

/// Shows `anchor`; clicking it toggles `content` in a popup below it. A click
/// outside the popup, Escape, or a click inside that triggers something (like
/// a link) closes it.
pub fn popup<'a, Message: 'a>(anchor: impl Into<Element<'a, Message>>, content: impl Into<Element<'a, Message>>) -> Popup<'a, Message> {
    Popup { anchor: anchor.into(), content: content.into() }
}

pub struct Popup<'a, Message> {
    anchor: Element<'a, Message>,
    content: Element<'a, Message>,
}

#[derive(Default)]
struct State {
    open: bool,
}

impl<Message> Widget<Message, Theme, Renderer> for Popup<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.anchor), Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.anchor.as_widget(), self.content.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.anchor.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if cursor.is_over(layout.bounds()) {
                let state = tree.state.downcast_mut::<State>();
                state.open = !state.open;
                shell.capture_event();
                shell.invalidate_layout();
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
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
        self.anchor.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        if !state.open {
            return None;
        }
        Some(overlay::Element::new(Box::new(Overlay {
            anchor: layout.bounds() + translation,
            content: &mut self.content,
            tree: &mut tree.children[1],
            open: &mut state.open,
        })))
    }
}

impl<'a, Message: 'a> From<Popup<'a, Message>> for Element<'a, Message> {
    fn from(p: Popup<'a, Message>) -> Self {
        Element::new(p)
    }
}

struct Overlay<'a, 'b, Message> {
    /// The anchor's bounds, in overlay coordinates.
    anchor: Rectangle,
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    open: &'b mut bool,
}

impl<Message> Overlay<'_, '_, Message> {
    fn close(&mut self, shell: &mut Shell<'_, Message>) {
        *self.open = false;
        shell.invalidate_layout();
        shell.request_redraw();
    }
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Overlay<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.content.as_widget_mut().layout(self.tree, renderer, &layout::Limits::new(Size::ZERO, bounds));
        let size = node.size();
        // Below the anchor if it fits, else above; kept inside the window.
        let x = self.anchor.x.min(bounds.width - size.width).max(0.0);
        let below = self.anchor.y + self.anchor.height + GAP;
        let y = if below + size.height <= bounds.height { below } else { (self.anchor.y - GAP - size.height).max(0.0) };
        node.move_to(Point::new(x, y))
    }

    fn draw(&self, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor) {
        self.content.as_widget().draw(self.tree, renderer, theme, style, layout, cursor, &layout.bounds());
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        match event {
            // Not captured, so the click still reaches what's under it. A
            // click on the anchor is left to the anchor, which toggles.
            Event::Mouse(mouse::Event::ButtonPressed(_)) if !cursor.is_over(layout.bounds()) => {
                if !cursor.is_over(self.anchor) {
                    self.close(shell);
                }
                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(key::Named::Escape), .. }) => {
                self.close(shell);
                shell.capture_event();
                return;
            }
            _ => {}
        }
        self.content.as_widget_mut().update(self.tree, event, layout, cursor, renderer, clipboard, shell, &layout.bounds());
        // A button inside was clicked.
        if matches!(event, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))) && shell.is_event_captured() {
            self.close(shell);
        }
    }

    fn mouse_interaction(&self, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(self.tree, layout, cursor, &layout.bounds(), renderer)
    }
}
