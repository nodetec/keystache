use iced::{widget::Column, Task};

use crate::{app, fedimint::FederationView, routes::ConnectedState};

pub mod add;
pub mod federation_details;
pub mod list;

#[derive(Debug, Clone)]
pub enum Message {
    Add(add::Message),
}

pub struct Page {
    pub connected_state: ConnectedState,
    pub subroute: Subroute,
}

impl Page {
    pub fn new(connected_state: &ConnectedState, subroute: Subroute) -> Self {
        Self {
            connected_state: connected_state.clone(),
            subroute: Subroute::List(list::Page::new(connected_state)),
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::Add(add_msg) => {
                if let Subroute::Add(add_page) = &mut self.subroute {
                    add_page.update(add_msg)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        match &self.subroute {
            Subroute::List(list) => list.view(),
            Subroute::Add(add) => add.view(),
            Subroute::FederationDetails(federation_details) => federation_details.view(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubrouteName {
    List,
    Add,
    FederationDetails(FederationView),
}

impl SubrouteName {
    pub fn to_default_subroute(&self, connected_state: &ConnectedState) -> Subroute {
        match self {
            Self::List => Subroute::List(list::Page::new(connected_state)),
            Self::FederationDetails(federation_view) => {
                Subroute::FederationDetails(federation_details::Page::new(federation_view.clone()))
            }
            Self::Add => Subroute::Add(add::Page::new(connected_state)),
        }
    }
}

pub enum Subroute {
    List(list::Page),
    Add(add::Page),
    FederationDetails(federation_details::Page),
}

impl Subroute {
    pub fn to_name(&self) -> SubrouteName {
        match self {
            Self::List(_) => SubrouteName::List,
            Self::Add(_) => SubrouteName::Add,
            Self::FederationDetails(federation_details) => {
                SubrouteName::FederationDetails(federation_details.federation_view().clone())
            }
        }
    }
}
