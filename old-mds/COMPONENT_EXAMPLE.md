# Creating Components with Child Components

## Overview

Your React-like architecture supports composing components with children through two main patterns:

1. **Container Elements** - Layout-based composition with automatic child positioning
2. **Fragment Elements** - Logical grouping of children without layout

## Pattern 1: Using Container Elements (Most Common)

Container elements automatically handle layout using ratatui's layout system.

### Example: Card Component with Header and Content

```rust
use crate::tui::framework::component::{vertical, horizontal, Component, Constraint, Element};
use ratatui::{buffer::Buffer, layout::Rect, widgets::{Block, Borders, Paragraph}};

/// Props for Card component
#[derive(Clone)]
pub struct CardProps {
    pub title: String,
    pub children: Vec<Element>,
    pub show_border: bool,
}

/// Card component - a container with optional border and title
pub struct Card;

impl Component for Card {
    type Props = CardProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        // Create vertical layout: [Header, Content]
        vertical(
            [
                Constraint::Length(3),  // Header height
                Constraint::Min(0),      // Content takes rest
            ],
            vec![
                // Header child
                self.render_header(&props.title, props.show_border),

                // Content children (passed via props)
                self.render_content(&props.children, props.show_border),
            ],
        )
    }
}

impl Card {
    fn render_header(&self, title: &str, show_border: bool) -> Element {
        Element::Widget(Box::new(HeaderWidget {
            title: title.to_string(),
            show_border,
        }))
    }

    fn render_content(&self, children: &[Element], show_border: bool) -> Element {
        if show_border {
            // Wrap children in a bordered container
            vertical(
                [Constraint::Min(0)],
                children.to_vec(),
            )
        } else {
            // Return children as fragment
            Element::Fragment(children.to_vec())
        }
    }
}

// Widget for header rendering
struct HeaderWidget {
    title: String,
    show_border: bool,
}

impl RenderableWidget for HeaderWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = if self.show_border {
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .title(self.title.as_str())
        } else {
            Block::default().title(self.title.as_str())
        };

        block.render(area, buf);
    }

    fn clone_box(&self) -> Box<dyn RenderableWidget> {
        Box::new(Self {
            title: self.title.clone(),
            show_border: self.show_border,
        })
    }
}
```

### Using the Card Component

```rust
use crate::tui::framework::component::Element;

// In your parent component's view() method:
fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
    let card_props = CardProps {
        title: "Game Details".to_string(),
        show_border: true,
        children: vec![
            // Child 1: Score display
            self.render_score_widget(&props.score),

            // Child 2: Period breakdown
            self.render_period_table(&props.periods),

            // Child 3: Status message
            Element::Widget(Box::new(StatusWidget {
                message: "Game is live!".to_string(),
            })),
        ],
    };

    Card.view(&card_props, &())
}
```

## Pattern 2: Nested Components (Component in Component)

You can nest components within each other for complex UIs.

### Example: Dashboard with Multiple Cards

```rust
/// Dashboard component with multiple card children
pub struct Dashboard;

#[derive(Clone)]
pub struct DashboardProps {
    pub games: Vec<GameData>,
    pub standings: Vec<TeamStanding>,
}

impl Component for Dashboard {
    type Props = DashboardProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        // Create 2-column layout
        horizontal(
            [
                Constraint::Percentage(50),  // Left column
                Constraint::Percentage(50),  // Right column
            ],
            vec![
                // Left: Games card
                self.render_games_card(&props.games),

                // Right: Standings card
                self.render_standings_card(&props.standings),
            ],
        )
    }
}

impl Dashboard {
    fn render_games_card(&self, games: &[GameData]) -> Element {
        // Create children for the games
        let game_children: Vec<Element> = games
            .iter()
            .map(|game| self.render_game_row(game))
            .collect();

        // Wrap in Card component
        let card_props = CardProps {
            title: "Today's Games".to_string(),
            show_border: true,
            children: game_children,
        };

        Card.view(&card_props, &())
    }

    fn render_standings_card(&self, standings: &[TeamStanding]) -> Element {
        // Create children for standings
        let standing_children: Vec<Element> = standings
            .iter()
            .map(|team| self.render_team_row(team))
            .collect();

        let card_props = CardProps {
            title: "Standings".to_string(),
            show_border: true,
            children: standing_children,
        };

        Card.view(&card_props, &())
    }

    fn render_game_row(&self, game: &GameData) -> Element {
        Element::Widget(Box::new(GameRowWidget {
            game: game.clone(),
        }))
    }

    fn render_team_row(&self, team: &TeamStanding) -> Element {
        Element::Widget(Box::new(TeamRowWidget {
            team: team.clone(),
        }))
    }
}
```

## Pattern 3: Dynamic Children Based on State

You can conditionally render children based on props or state.

### Example: Collapsible Section

```rust
#[derive(Clone)]
pub struct CollapsibleProps {
    pub title: String,
    pub expanded: bool,
    pub children: Vec<Element>,
}

pub struct Collapsible;

impl Component for Collapsible {
    type Props = CollapsibleProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        if props.expanded {
            // Show header + children
            vertical(
                [
                    Constraint::Length(1),  // Header
                    Constraint::Min(0),      // Content
                ],
                vec![
                    self.render_header(&props.title, true),
                    Element::Fragment(props.children.clone()),
                ],
            )
        } else {
            // Show only collapsed header
            vertical(
                [Constraint::Length(1)],
                vec![self.render_header(&props.title, false)],
            )
        }
    }
}

impl Collapsible {
    fn render_header(&self, title: &str, expanded: bool) -> Element {
        let icon = if expanded { "▼" } else { "▶" };
        Element::Widget(Box::new(CollapsibleHeaderWidget {
            title: format!("{} {}", icon, title),
        }))
    }
}
```

## Pattern 4: List Component with Item Children

Create reusable list components that render items uniformly.

### Example: Scrollable List

```rust
#[derive(Clone)]
pub struct ListProps<T: Clone> {
    pub items: Vec<T>,
    pub selected_index: Option<usize>,
    pub render_item: fn(&T, bool) -> Element,
}

pub struct List<T: Clone> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + 'static> Component for List<T> {
    type Props = ListProps<T>;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        let children: Vec<Element> = props
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_selected = props.selected_index == Some(idx);
                (props.render_item)(item, is_selected)
            })
            .collect();

        vertical(
            vec![Constraint::Length(1); children.len()],
            children,
        )
    }
}

// Usage:
fn render_team_list(teams: &[Team], selected: Option<usize>) -> Element {
    let list_props = ListProps {
        items: teams.to_vec(),
        selected_index: selected,
        render_item: |team, is_selected| {
            Element::Widget(Box::new(TeamItemWidget {
                name: team.name.clone(),
                selected: is_selected,
            }))
        },
    };

    List { _phantom: std::marker::PhantomData }.view(&list_props, &())
}
```

## Key Patterns in Your Architecture

### 1. **Composition Over Inheritance**
Components compose other components through props, not inheritance.

### 2. **Children as Props**
Children are passed via props (`children: Vec<Element>`), not as special syntax.

### 3. **Layout via Containers**
Use `vertical()` and `horizontal()` helpers for automatic layout:
- `vertical([constraints], [children])` - Stack children vertically
- `horizontal([constraints], [children])` - Arrange children horizontally

### 4. **Constraint Types**
- `Constraint::Length(n)` - Fixed height/width
- `Constraint::Percentage(n)` - Percentage of parent
- `Constraint::Min(n)` - Minimum size, grow as needed
- `Constraint::Max(n)` - Maximum size
- `Constraint::Ratio(n, d)` - Ratio (n/d) of parent

### 5. **Fragment for Non-Visual Grouping**
Use `Element::Fragment(children)` when you want to group without layout.

## Real Example from Your Codebase

Here's how the `App` component composes children:

```rust
// From src/tui/components/app.rs
impl Component for App {
    type Props = AppState;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        vertical(
            [
                Constraint::Length(1), // TabBar (child 1)
                Constraint::Min(0),    // Content (child 2)
                Constraint::Length(1), // StatusBar (child 3)
            ],
            vec![
                // Child 1: TabBar component
                TabBar.view(&props.navigation.current_tab, &()),

                // Child 2: Dynamic content (tab or panel)
                self.render_content(props),

                // Child 3: StatusBar component
                StatusBar.view(&props.system, &()),
            ],
        )
    }
}
```

And `ScoresTab` composes its own children:

```rust
// From src/tui/components/scores_tab.rs
impl Component for ScoresTab {
    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        vertical(
            [
                Constraint::Length(2), // Date selector (child 1)
                Constraint::Min(0),    // Game list (child 2)
            ],
            vec![
                // Child 1: Date selector widget
                self.render_date_selector(props),

                // Child 2: Game list widget
                self.render_game_list(props),
            ],
        )
    }
}
```

## Testing Components with Children

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_renders_children() {
        let card = Card;
        let props = CardProps {
            title: "Test Card".to_string(),
            show_border: true,
            children: vec![
                Element::None,
                Element::None,
            ],
        };

        let element = card.view(&props, &());

        // Verify it's a container with children
        match element {
            Element::Container { children, .. } => {
                assert_eq!(children.len(), 2); // Header + content
            }
            _ => panic!("Expected container element"),
        }
    }
}
```

## Summary

Your React-like architecture supports component composition through:

1. **`vertical()` / `horizontal()`** - Layout-based composition
2. **`Element::Fragment`** - Logical grouping without layout
3. **Children as Props** - Pass `Vec<Element>` via props
4. **Nested Components** - Components render other components
5. **Dynamic Children** - Conditional rendering based on state

This gives you the same composability as React's JSX, but in Rust with type safety!
